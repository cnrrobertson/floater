# floater

A headless [Zellij](https://zellij.dev) plugin that manages floating panes for arbitrary commands — with configurable position, size, toggle vs. always-open mode, per-instance staggering, and optional cwd inheritance from the focused pane.

## Features

- **Toggle mode** — press a key to open; press again to focus the existing pane
- **Always-open mode** — each keypress opens a new staggered floating instance
- **Staggering** — additional instances offset by configurable rows/cols so they don't perfectly overlap
- **Focused cwd** — optionally opens the command in the currently focused pane's working directory
- **Auto-close** — floating pane closes automatically when the command exits (no blank rerun prompt)
- **Headless** — no visible plugin pane; runs silently in the background

## Requirements

- Zellij 0.44.x (plugin API is version-specific; other versions are not supported)
- Rust + `wasm32-wasip1` target (`rustup target add wasm32-wasip1`) — only needed to build from source

## Installation

```bash
git clone https://github.com/cnrrobertson/floater
cd floater
cargo build --release --target wasm32-wasip1
mkdir -p ~/.config/zellij/plugins
cp target/wasm32-wasip1/release/floater.wasm ~/.config/zellij/plugins/floater.wasm
```

Or with [just](https://github.com/casey/just):

```bash
just install
```

## Configuration

Add to your `~/.config/zellij/config.kdl`:

### 1. Load the plugin

Commands are configured as TOML sections inside a single `config` string. Add a `[name]` section for each command — no code changes or rebuilds needed to add new ones.

```kdl
load_plugins {
    "file:~/.config/zellij/plugins/floater.wasm" {
        _allow_exec_host_cmd true
        config r#"
[lazygit]
cmd = "lazygit"
x = "8%"
y = "8%"
w = "80%"
h = "80%"
stagger_x = 2
stagger_y = 1
max_stagger = 5
mode = "toggle"
cwd = "focused"

[yazi]
cmd = "yazi"
x = "5%"
y = "5%"
w = "90%"
h = "90%"
stagger_x = 3
stagger_y = 2
max_stagger = 5
mode = "toggle"
cwd = "focused_arg"
        "#
    }
}
```

### 2. Add keybindings

```kdl
shared_among "normal" "locked" {
    bind "Alt ," {
        MessagePlugin {
            name    "toggle"
            payload "cmd=lazygit"
        }
    }
    bind "Alt /" {
        MessagePlugin {
            name    "toggle"
            payload "cmd=yazi"
        }
    }
}
```

## Config key reference

Each `[name]` section supports the following keys:

| Key | Description | Default |
|-----|-------------|---------|
| `cmd` | Executable to run (required) | — |
| `args` | Array of arguments, e.g. `["--config", "/path"]` | `[]` |
| `x` | X position: `"N%"` or `"N"` cols | `"10%"` |
| `y` | Y position: `"N%"` or `"N"` rows | `"10%"` |
| `w` | Width: `"N%"` or `"N"` cols | `"80%"` |
| `h` | Height: `"N%"` or `"N"` rows | `"80%"` |
| `stagger_x` | Cols to shift right per extra open instance | `2` |
| `stagger_y` | Rows to shift down per extra open instance | `1` |
| `max_stagger` | Wrap stagger index at this count | `5` |
| `mode` | `"toggle"` or `"open"` | `"toggle"` |
| `cwd` | `"focused"` = use focused pane's cwd; `"focused_arg"` = also pass cwd as first arg (e.g. yazi); `""` = none | `""` |

## Pipe actions

You can send pipe messages to control floater from keybindings:

| Action | Payload | Behavior |
|--------|---------|----------|
| `toggle` | `cmd=NAME` | Focus existing (toggle mode) or open new |
| `open` | `cmd=NAME` | Always open a new instance |
| `close` | `cmd=NAME` | Close the most recent instance |
| `closeall` | `cmd=NAME` | Close all open instances |

## Building from source

```bash
# requires rustup
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
```
