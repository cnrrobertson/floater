cargo := `rustup which cargo`
rustc := `rustup which rustc`

install_dir := env('HOME') / ".config/zellij/plugins"
wasm := "target/wasm32-wasip1/release/floater.wasm"

build:
    RUSTC={{rustc}} {{cargo}} build --release --target wasm32-wasip1

build-dev:
    RUSTC={{rustc}} {{cargo}} build --target wasm32-wasip1

# Build and install to ~/.config/zellij/plugins/
install: build
    mkdir -p {{install_dir}}
    cp {{wasm}} {{install_dir}}/floater.wasm

# Reload the plugin in the current zellij session (after install)
reload:
    zellij action start-or-reload-plugin file:{{install_dir}}/floater.wasm
