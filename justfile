cargo := `rustup which cargo`
rustc := `rustup which rustc`

install_dir := env('HOME') / ".config/zellij/plugins"
# Build outside of any OneDrive-synced folder. OneDrive on macOS treats files
# in `target/` as cloud placeholders and blocks the hardlink/rename dance
# cargo does mid-build ("Operation not permitted"), so we keep the source
# tree in OneDrive but redirect build output to local, unsynced disk.
target_dir := env('HOME') / ".cache/cargo-target/floater"
wasm := target_dir / "wasm32-wasip1/release/floater.wasm"

build:
    CARGO_TARGET_DIR={{target_dir}} RUSTC={{rustc}} {{cargo}} build --release --target wasm32-wasip1

build-dev:
    CARGO_TARGET_DIR={{target_dir}} RUSTC={{rustc}} {{cargo}} build --target wasm32-wasip1

# Build and install to ~/.config/zellij/plugins/
install: build
    mkdir -p {{install_dir}}
    cp {{wasm}} {{install_dir}}/floater.wasm

# Reload the plugin in the current zellij session (after install)
reload:
    zellij action start-or-reload-plugin file:{{install_dir}}/floater.wasm
