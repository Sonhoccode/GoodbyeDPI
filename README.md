# GoodbyeDPI Manager by Sonhoccode

A Windows desktop app by **[Sonhoccode](https://github.com/Sonhoccode)** for installing and managing GoodbyeDPI. This is a community fork of [ValdikSS's GoodbyeDPI](https://github.com/ValdikSS/GoodbyeDPI), not an official release from the original author.

## Download and install

Get the installer from the [latest release](https://github.com/Sonhoccode/GoodbyeDPI/releases/latest). The current release is [GoodbyeDPI Manager 1.01](https://github.com/Sonhoccode/GoodbyeDPI/releases/tag/v1.01).

Run `GoodbyeDPI-Manager-All-in-One-1.01.exe` on Windows 10/11 x64 and approve the Administrator prompt. The installer bundles GoodbyeDPI 0.2.2, WinDivert, and the manager app. If a `GoodbyeDPI` service already exists, the installer leaves that service unchanged.

## Features

- Start, stop, and monitor the GoodbyeDPI service.
- Choose the `-5` or `-6` preset. Stop the service before applying a new preset.
- Set whether the service starts with Windows.
- Switch between English (default), Vietnamese, Spanish, and Russian. The language choice is saved.
- Check for a newer release when the app opens. If one is available, choose **Install** to download the installer, verify its SHA-256, and run it. Updates are not installed silently.
- Uninstall from the Settings page.

GoodbyeDPI may not work on every network. Managing the Windows service requires Administrator privileges. This release is not commercially code-signed.

## Buy me a coffee

Support is optional and does not affect any app features. The same QR code is available from the **Buy me a coffee** button in the app.

<details>
<summary>Show VietQR support code</summary>

<img src="manager/assets/coffee.jpg" alt="VietQR support code for Sonhoccode" width="320">

</details>

## Source and credits

The Rust app, icon, and installer are in [`manager/`](manager/). See its [build instructions](manager/README.md).

The GoodbyeDPI engine is by [ValdikSS](https://github.com/ValdikSS/GoodbyeDPI), and WinDivert is by [basil00](https://github.com/basil00/Divert). Their licenses are included in [`manager/engine/licenses/`](manager/engine/licenses/). This fork retains the upstream engine's source history and license.
