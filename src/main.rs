use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use zellij_tile::prelude::*;
use zellij_utils::input::layout::PercentOrFixed;

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct State {
    /// Parsed per-command configuration, keyed by command name (e.g. "lazygit")
    commands: HashMap<String, CommandConfig>,
    /// Currently open pane IDs per (tab_index, command name)
    open_panes: HashMap<(usize, String), Vec<u32>>,
    /// CWD of the currently focused pane, updated via CwdChanged events
    focused_pane_cwd: PathBuf,
    /// The pane ID that currently has focus (updated via PaneUpdate)
    focused_pane_id: Option<PaneId>,
    /// Last known focused tab index (fallback cache updated via PaneUpdate)
    focused_tab: usize,
}

#[derive(Clone)]
struct CommandConfig {
    executable: String,
    args: Vec<String>,
    x: CoordValue,
    y: CoordValue,
    width: CoordValue,
    height: CoordValue,
    /// Fixed cols to shift right per additional open window
    stagger_x: usize,
    /// Fixed rows to shift down per additional open window
    stagger_y: usize,
    /// Wrap the stagger slot index at this count (default 5)
    max_stagger: usize,
    mode: OpenMode,
    /// If true, open the command in the focused pane's cwd
    use_focused_cwd: bool,
    /// If true, also pass the resolved cwd as the first positional argument
    /// (needed for commands like yazi that use argv[1] rather than process cwd)
    cwd_as_arg: bool,
}

#[derive(Clone)]
enum CoordValue {
    Percent(usize),
    Fixed(usize),
}

#[derive(Clone, PartialEq)]
enum OpenMode {
    /// Focus the most-recent open instance instead of opening a new one
    Toggle,
    /// Always open a new staggered instance
    AlwaysOpen,
}

// ─── Plugin registration ──────────────────────────────────────────────────────

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, config: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::RunCommands,
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[
            EventType::CommandPaneOpened,
            EventType::CommandPaneExited,
            EventType::PaneUpdate,
            EventType::CwdChanged,
            EventType::PermissionRequestResult,
        ]);
        self.commands = parse_config(&config);
        // Default cwd to home so it's never an empty path
        if let Some(home) = std::env::var_os("HOME") {
            self.focused_pane_cwd = PathBuf::from(home);
        }
        hide_self();
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::CommandPaneOpened(pane_id, ctx) => {
                if let (Some(name), Some(tab_str)) =
                    (ctx.get("floater_cmd"), ctx.get("floater_tab"))
                {
                    let tab: usize = tab_str.parse().unwrap_or(0);
                    self.open_panes
                        .entry((tab, name.clone()))
                        .or_default()
                        .push(pane_id);
                }
            }
            Event::CommandPaneExited(pane_id, _exit_code, ctx) => {
                if let (Some(name), Some(tab_str)) =
                    (ctx.get("floater_cmd"), ctx.get("floater_tab"))
                {
                    let tab: usize = tab_str.parse().unwrap_or(0);
                    let key = (tab, name.clone());
                    if let Some(ids) = self.open_panes.get_mut(&key) {
                        ids.retain(|&id| id != pane_id);
                    }
                    close_terminal_pane(pane_id);
                }
            }
            Event::CwdChanged(_pane_id, new_cwd, _client_ids) => {
                self.focused_pane_cwd = new_cwd;
            }
            Event::PaneUpdate(manifest) => {
                for (tab_idx, panes) in &manifest.panes {
                    for pane in panes {
                        if pane.is_focused && !pane.is_plugin {
                            self.focused_tab = *tab_idx;
                            self.focused_pane_id = Some(PaneId::Terminal(pane.id));
                        }
                    }
                }
            }
            Event::PermissionRequestResult(_) => {
                hide_self();
            }
            _ => {}
        }
        false // headless — never triggers render
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        let name = pipe_message.name.as_str();
        let payload = pipe_message.payload.as_deref().unwrap_or("");
        let cmd_name = payload
            .strip_prefix("cmd=")
            .unwrap_or("")
            .trim()
            .to_string();

        if cmd_name.is_empty() {
            return false;
        }

        match name {
            "open"     => self.do_open(&cmd_name),
            "toggle"   => self.do_toggle(&cmd_name),
            "close"    => self.do_close(&cmd_name),
            "closeall" => self.do_closeall(&cmd_name),
            _ => {}
        }
        false // headless
    }

    fn render(&mut self, _rows: usize, _cols: usize) {
        // Intentionally empty — this is a headless background plugin.
    }
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

impl State {
    /// Returns the currently focused (tab_index, PaneId).
    /// Queries zellij synchronously; falls back to cached values from PaneUpdate.
    fn current_focus(&self) -> (usize, Option<PaneId>) {
        match get_focused_pane_info() {
            Ok((tab, pane_id)) => (tab, Some(pane_id)),
            Err(_) => (self.focused_tab, self.focused_pane_id),
        }
    }

    /// Returns the currently focused tab index.
    fn current_tab(&self) -> usize {
        self.current_focus().0
    }

    /// Open or focus depending on the command's configured mode.
    fn do_toggle(&mut self, name: &str) {
        let tab = self.current_tab();
        let key = (tab, name.to_string());
        let mode = self.commands.get(name).map(|c| c.mode.clone());
        let open_count = self.open_panes.get(&key).map(|v| v.len()).unwrap_or(0);

        if mode == Some(OpenMode::Toggle) && open_count > 0 {
            if let Some(&id) = self.open_panes[&key].last() {
                focus_terminal_pane(id, true, false);
            }
        } else {
            self.do_open(name);
        }
    }

    /// Always open a new staggered floating pane.
    fn do_open(&mut self, name: &str) {
        let Some(config) = self.commands.get(name).cloned() else {
            return;
        };

        let (tab, live_pane_id) = self.current_focus();
        let key = (tab, name.to_string());
        let open_count = self.open_panes.get(&key).map(|v| v.len()).unwrap_or(0);
        let slot = open_count % config.max_stagger;
        let dx = slot * config.stagger_x;
        let dy = slot * config.stagger_y;

        let coords = FloatingPaneCoordinates {
            x:          Some(apply_offset(&config.x, dx)),
            y:          Some(apply_offset(&config.y, dy)),
            width:      Some(to_pfixed(&config.width)),
            height:     Some(to_pfixed(&config.height)),
            pinned:     None,
            borderless: None,
        };

        let cwd = if config.use_focused_cwd {
            // Query live cwd from the focused pane; fall back to cached value
            let pane_id = live_pane_id.or(self.focused_pane_id);
            let live_cwd = pane_id.and_then(|pid| get_pane_cwd(pid).ok());
            Some(live_cwd.unwrap_or_else(|| self.focused_pane_cwd.clone()))
        } else {
            None
        };

        let mut ctx = BTreeMap::new();
        ctx.insert("floater_cmd".to_string(), name.to_string());
        ctx.insert("floater_tab".to_string(), tab.to_string());

        // Build args: optionally prepend cwd as first positional arg
        let mut args = config.args.clone();
        if config.cwd_as_arg {
            if let Some(ref cwd_path) = cwd {
                args.insert(0, cwd_path.to_string_lossy().to_string());
            }
        }

        let cmd = CommandToRun {
            path: PathBuf::from(&config.executable),
            args,
            cwd,
        };

        open_command_pane_floating(cmd, Some(coords), ctx);
    }

    /// Close the most-recently opened instance on the current tab.
    fn do_close(&mut self, name: &str) {
        let key = (self.current_tab(), name.to_string());
        if let Some(ids) = self.open_panes.get_mut(&key) {
            if let Some(id) = ids.pop() {
                close_terminal_pane(id);
            }
        }
    }

    /// Close all open instances of a command on the current tab.
    fn do_closeall(&mut self, name: &str) {
        let key = (self.current_tab(), name.to_string());
        if let Some(ids) = self.open_panes.remove(&key) {
            for id in ids {
                close_terminal_pane(id);
            }
        }
    }
}

// ─── Config parsing ────────────────────────────────────────────────────────────

/// TOML shape: top-level table where each key is a named command.
#[derive(Deserialize)]
struct TomlConfig {
    #[serde(flatten)]
    commands: HashMap<String, TomlCommand>,
}

#[derive(Deserialize)]
struct TomlCommand {
    cmd: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    x: Option<String>,
    #[serde(default)]
    y: Option<String>,
    #[serde(default)]
    w: Option<String>,
    #[serde(default)]
    h: Option<String>,
    #[serde(default)]
    stagger_x: Option<usize>,
    #[serde(default)]
    stagger_y: Option<usize>,
    #[serde(default)]
    max_stagger: Option<usize>,
}

/// Parse the KDL plugin block. Expects a single `config` key containing a
/// TOML string with one section per named command.
///
/// Example KDL:
/// ```kdl
/// config r#"
///   [lazygit]
///   cmd = "lazygit"
///   mode = "toggle"
///   x = "10%"; y = "5%"; w = "80%"; h = "90%"
///
///   [yazi]
///   cmd = "yazi"
///   cwd = "focused_arg"
/// "#
/// ```
fn parse_config(config: &BTreeMap<String, String>) -> HashMap<String, CommandConfig> {
    let Some(toml_str) = config.get("config") else {
        return HashMap::new();
    };

    let parsed: TomlConfig = match toml::from_str(toml_str) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    let mut result = HashMap::new();
    for (name, tc) in parsed.commands {
        let mut parts = tc.cmd.split_whitespace();
        let executable = parts.next().unwrap_or("").to_string();
        if executable.is_empty() {
            continue;
        }
        let mut args: Vec<String> = parts.map(String::from).collect();
        args.extend(tc.args);

        let mode = match tc.mode.as_deref().map(str::to_lowercase).as_deref() {
            Some("open") | Some("alwaysopen") => OpenMode::AlwaysOpen,
            _ => OpenMode::Toggle,
        };

        let cwd_val = tc.cwd.as_deref().map(str::to_lowercase).unwrap_or_default();
        let use_focused_cwd = cwd_val == "focused" || cwd_val == "focused_arg";
        let cwd_as_arg = cwd_val == "focused_arg";

        let max_stagger = match tc.max_stagger.unwrap_or(5) {
            0 => 5,
            v => v,
        };

        result.insert(
            name,
            CommandConfig {
                executable,
                args,
                x:          parse_coord(tc.x.as_deref().unwrap_or("")),
                y:          parse_coord(tc.y.as_deref().unwrap_or("")),
                width:      parse_coord(tc.w.as_deref().unwrap_or("")),
                height:     parse_coord(tc.h.as_deref().unwrap_or("")),
                stagger_x:  tc.stagger_x.unwrap_or(2),
                stagger_y:  tc.stagger_y.unwrap_or(1),
                max_stagger,
                mode,
                use_focused_cwd,
                cwd_as_arg,
            },
        );
    }

    result
}

// ─── Coordinate helpers ────────────────────────────────────────────────────────

/// Parse "80%" → Percent(80) or "120" → Fixed(120).
fn parse_coord(s: &str) -> CoordValue {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        let v: usize = pct.trim().parse().unwrap_or(50);
        CoordValue::Percent(v.min(100))
    } else {
        CoordValue::Fixed(s.parse().unwrap_or(0))
    }
}

/// Apply a stagger offset to a coordinate.
///
/// - Percent base → stays Percent, offset adds percentage points (capped at 90%).
/// - Fixed base → adds offset in terminal cells.
fn apply_offset(c: &CoordValue, offset: usize) -> PercentOrFixed {
    match c {
        CoordValue::Percent(p) => PercentOrFixed::Percent((p + offset).min(90)),
        CoordValue::Fixed(n) => PercentOrFixed::Fixed(n + offset),
    }
}

/// Convert a `CoordValue` to `PercentOrFixed` with no offset (used for width/height).
fn to_pfixed(c: &CoordValue) -> PercentOrFixed {
    apply_offset(c, 0)
}
