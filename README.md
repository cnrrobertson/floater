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

```kdl
load_plugins {
    "file:/Users/YOU/.config/zellij/plugins/floater.wasm" {
        _allow_exec_host_cmd true

        lazygit_cmd         "lazygit"
        lazygit_x           "8%"
        lazygit_y           "8%"
        lazygit_w           "80%"
        lazygit_h           "80%"
        lazygit_stagger_x   "2"
        lazygit_stagger_y   "1"
        lazygit_max_stagger "5"
        lazygit_mode        "toggle"
        lazygit_cwd         "focused"

        yazi_cmd            "yazi"
        yazi_x              "5%"
        yazi_y              "5%"
        yazi_w              "90%"
        yazi_h              "90%"
        yazi_stagger_x      "3"
        yazi_stagger_y      "2"
        yazi_max_stagger    "5"
        yazi_mode           "toggle"
        yazi_cwd            "focused"
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

Each command is configured with keys prefixed by its name (e.g. `lazygit_`):

| Key | Description | Default |
|-----|-------------|---------|
| `{name}_cmd` | Executable to run (required) | — |
| `{name}_args` | Space-separated arguments | `""` |
| `{name}_x` | X position: `"N%"` or `"N"` cols | `"10%"` |
| `{name}_y` | Y position: `"N%"` or `"N"` rows | `"10%"` |
| `{name}_w` | Width: `"N%"` or `"N"` cols | `"80%"` |
| `{name}_h` | Height: `"N%"` or `"N"` rows | `"80%"` |
| `{name}_stagger_x` | Cols to shift right per extra open instance | `2` |
| `{name}_stagger_y` | Rows to shift down per extra open instance | `1` |
| `{name}_max_stagger` | Wrap stagger index at this count | `5` |
| `{name}_mode` | `"toggle"` or `"open"` | `"toggle"` |
| `{name}_cwd` | `"focused"` = use focused pane's cwd; `""` = none | `""` |

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
