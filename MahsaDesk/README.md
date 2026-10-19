# MahsaDesk

Desktop port of the [MahsaNG](https://github.com/GFW-knocker/MahsaNG) (v2rayNG fork)
client for Windows. A decentralized, non-profit client where everyone can share their
configs to help others reach a free internet.

![logo](../Mahsa_logo.jpg)

## Features (v0.1)

- **Xray core** (official XTLS binary) managed by the app - SOCKS 10808 / HTTP 10809
- **System proxy mode** - automates the Windows system proxy on connect/disconnect
- **TUN mode (experimental)** - full system VPN via bundled sing-box + wintun, requires
  admin (UAC) approval; stop it from the tray/Stop button
- **Share-link import**: vmess / vless (REALITY, XTLS-vision) / trojan / shadowsocks /
  socks / http, plus base64 subscription payloads and clipboard import
- **Subscriptions** - add URLs, manual and timed auto-update
- **TLS fragment** settings (packets / length / interval) wired into the Xray TLS dialer
- **CDN IP rewrite** - Manually replace the connect address with a Cloudflare IP while
  keeping SNI/Host (same idea as Mahsang's "config IP")
- **Routing presets** - proxy everything / bypass Iran (+China) with ad blocking, plus
  per-domain overrides; geoip/geosite data auto-download
- **DNS settings** (remote DoH/UDP), sniffing, MUX, allow-insecure, IPv6 preference
- **System tray** - connect/disconnect, open, quit; minimize-to-tray; auto launch at login;
  auto-connect at startup
- **Live core logs** and session traffic counters
- **Persian (RTL) and English** interface
- **Config export/import** of the whole pool (servers + subs + settings) as JSON

## What stays out (by design)

MahsaNG's donated-config service depends on closed-source parts (config encryption and
mahsaserver authentication) that are not available here. Use your own configs or any
public subscription.

## Build from source

Requirements: Node 22+, Rust (MSVC), and the Visual Studio C++ build tools.

```powershell
cd MahsaDesk
npm install
# fetch xray.exe / sing-box.exe / wintun.dll / geo files into src-tauri/binaries
powershell -NoProfile -ExecutionPolicy Bypass -File .scripts/download-binaries.ps1
npm run tauri build
```

The NSIS installer lands in `src-tauri/target/release/bundle/nsis/`; the portable exe is
`src-tauri/target/release/MahsaDesk.exe` (keep the `binaries` folder next to it).

CI builds and publishes releases automatically on `v*` tags (see
`.github/workflows/release.yml`).

## Privacy

The app talks only to your proxy servers and your subscription URLs. No analytics, no
telemetry. Configuration lives in `%APPDATA%\com.mahsadesk.desktop`.
