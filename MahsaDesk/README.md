# MahsaDesk

Desktop port of the [MahsaNG](https://github.com/GFW-knocker/MahsaNG) (v2rayNG fork) client for Windows. A decentralized, non-profit client where everyone can share their configs to help others reach a free internet.

![MahsaDesk Logo](icons/icon.ico)

## 📋 Overview

MahsaDesk is a Windows desktop application that provides a user-friendly interface for managing V2Ray/Xray proxy configurations. It enables users to:

- Connect to proxy servers via Xray core
- Manage subscription URLs to automatically fetch server configs
- Configure advanced routing, DNS, and encryption settings
- Use Psiphon tunneling in addition to proxy configs
- Perform network diagnostics and speed tests
- Customize system proxy and TUN VPN modes

The app is available as both an NSIS installer and a portable executable.

## ✨ Features

### Core Features (v0.3.2)

- **Xray core** (official XTLS binary) managed by the app - SOCKS 10808 / HTTP 10809
- **System proxy mode** - automates the Windows system proxy on connect/disconnect
- **TUN mode (experimental)** - full system VPN via bundled sing-box + wintun, requires admin (UAC) approval
- **Psiphon integration** - runs psiphon-tunnel-core.exe as standalone tunnel ("only" mode) or chains on top of active config ("after" mode)
- **Share-link import**: Supports vmess/vless (REALITY, XTLS-vision), trojan, shadowsocks, socks, http, hysteria2, and base64 subscription payloads
- **QR tools**: Decode QR codes from clipboard screenshots, capture screen regions and import configs, show profiles as QR codes
- **Test tools**: TCP ping, real URL latency test through a throwaway core instance (per-server and batched), config validation tool
- **Per-app routing (TUN)**: Pick which running programs bypass the tunnel or are the only ones routed through it
- **Subscriptions** - add URLs with manual and timed auto-update
- **TLS fragment** settings (packets/length/interval) wired into the Xray TLS dialer
- **CDN IP rewrite** - manually replace connect address with Cloudflare IP while keeping SNI/Host
- **Routing presets** - proxy everything / bypass Iran (+China) with ad blocking, plus per-domain overrides
- **Geo assets manager** - view, update and delete geoip.dat/geosite.dat
- **DNS settings** (remote DoH/UDP), sniffing, MUX, allow-insecure, IPv6 preference
- **System tray** - connect/disconnect, open, quit; minimize-to-tray; auto launch at login; auto-connect at startup
- **Live core logs** and session traffic counters
- **Hysteria2 support** - parse hysteria2:// / hy2:// links; run natively through TUN engine
- **Deep link import** - `mahsadesk://` URL scheme to hand configs to the app
- **Persian (RTL) and English** interface with automatic direction switching
- **Config export/import** of the whole pool (servers + subs + settings) as JSON

### New in v0.3.2

- **Startup update check** - silently compares version against GitHub release feed; shows banner with direct download link when newer version available (EN + FA)
- **Green/red switch colors** - all on/off switches and connection status dot use fixed brand colors (#01ca7a on / #f03b4c off) in both themes

### New in v0.3.1

- **Light / dark / auto theme** selection; `auto` follows the OS color scheme
- **Requirements manager** - checks every engine requirement (Xray, sing-box, wintun, geo data, Psiphon engine) against upstream, shows versions, and upgrades them in place
- **Complete builds** - installer and portable zip bundle every runtime requirement, including `psiphon-tunnel-core.exe`

### What stays out (by design)

MahsaNG's donated-config service depends on closed-source parts (config encryption and mahsaserver authentication) that are not available here. Use your own configs or any public subscription.

## 🛠️ Build from source

```powershell
cd MahsaDesk
npm install
# fetch xray.exe / sing-box.exe / wintun.dll / geo files into src-tauri/binaries
powershell -NoProfile -ExecutionPolicy Bypass -File .scripts/download-binaries.ps1
npm run tauri build
```

The NSIS installer lands in `src-tauri/target/release/bundle/nsis/`; the portable exe is `src-tauri/target/release/MahsaDesk.exe` (keep the `binaries` folder next to it).

CI builds and publishes releases automatically on `v*` tags.

## 🔒 Privacy

The app talks only to your proxy servers and your subscription URLs. No analytics, no telemetry. Configuration lives in `%APPDATA%\com.mahsadesk.desktop`.

## 📦 Reference Projects

This project is inspired by and builds upon the following open-source projects:

- **[MahsaNG](https://github.com/GFW-knocker/MahsaNG)** - The Android v2rayNG fork that serves as the foundation for this desktop port
- **[v2rayNG](https://github.com/2dust/v2rayNG)** - The original Android V2Ray client
- **[Xray-core](https://github.com/XTLS/Xray-core)** - The core proxy engine used by MahsaDesk
- **[sing-box](https://github.com/sing-box/sing-box)** - The universal proxy platform used for TUN mode
- **[psiphon-tunnel-core](https://github.com/Psiphon-Inc/psiphon-tunnel-core)** - The Psiphon tunnel engine integrated into MahsaDesk
- **[@tauri-apps](https://tauri.app)** - The framework used for this desktop application

## 🌐 Language Support

- **English** (default)
- **Persian (Farsi)** - Full RTL support with complete UI translation

## 📄 License

This project is open source and available under the MIT License.