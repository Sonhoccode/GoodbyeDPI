# GoodbyeDPI Manager

Windows desktop manager for the GoodbyeDPI service. This is a community fork, not an official ValdikSS release.

## Install

Download `GoodbyeDPI-Manager-All-in-One-1.00.exe` from this repository's Releases. The installer includes the GoodbyeDPI 0.2.2 engine and WinDivert, creates the Windows service, and installs the manager. Administrator permission is required.

The manager provides start/stop, startup setting, `-5`/`-6` presets, English/Vietnamese/Spanish/Russian UI, uninstall, release update checks, and a voluntary support QR. The QR image is included in the application binary. Each new release must include an installer asset and its SHA-256 in the release body as `SHA256: <64 hex characters>` for the updater to accept it.

## Build

Install Rust and Inno Setup 6. Run `cargo test` and `cargo build --release` in this directory, then compile `installer/GoodbyeDPIManager.iss` with `ISCC.exe`. The installer script expects the `engine/` files to remain present.

The GoodbyeDPI engine and WinDivert retain their own licenses in `engine/licenses/`.
