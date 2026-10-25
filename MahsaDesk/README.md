# MahsaDesk

Desktop port of the [MahsaNG](https://github.com/GFW-knocker/MahsaNG) (v2rayNG fork) client for Windows. A decentralized, non-profit client where everyone can share their configs to help others reach a free internet.

![MahsaDesk Logo](../Mahsa_logo.jpg)

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

### Core Features

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

### New in v0.3.6

- **Security and privacy audit** - a full audit of the app with every finding fixed: strict Content-Security-Policy, scoped opener permission, TLS certificate verification enabled everywhere ("Allow insecure TLS" now defaults off for new installs), the elevated TUN launch made injection-safe, a 32 MB cap on subscription downloads, and CI actions pinned to commit SHAs
- **Token-protected sharing** - the share-as-subscription URL now carries an unguessable 128-bit path, so only people you hand the link or QR to can pull your configs (re-share to get a new link)
- **Honest privacy section** - both READMEs now list every third party the app can contact, and "Share proxy on LAN" warns that the proxy is password-less while it is on

### New in v0.3.5

- **Per-app gate in system-proxy mode** - bypass/allow rules now work without TUN: the app owns the port the system proxy points at, identifies each client process by PID and forwards or dials per your rules - no admin rights, no kernel driver
- **Hysteria2 without admin** - hy2 profiles now connect in system-proxy mode too, through a non-elevated sing-box sidecar (previously TUN only)
- **Verified engine binaries** - every shipped engine gets a SHA-256 recorded when the build was made; the Requirements card shows verified / not verified and still upgrades engines in place
- **Smarter Servers view** - live search, sort (default / ping / name / protocol / subscription / newest), per-subscription filter, and bulk select to ping, real-test, copy, share or delete many servers at once
- **Share your pool as a subscription** - one click starts a local HTTP endpoint (custom port, LAN toggle, QR code) that phones and other PCs can add as a subscription URL
- **Clash and sing-box import** - paste a Clash YAML or sing-box JSON config and its proxies are extracted; curated community subscription sources are one click away
- **Connection doctor** - right after connecting, six checks (core process, SOCKS/HTTP ports, tunnel forwarding, exit IP, DNS) say exactly which link is broken
- **Better logs** - level filter, full-text search, copy all, export to file, and a crash history of abnormal engine exits
- **Per-ISP anti-filtering presets** - one-click TLS-fragment presets for MCI/Hiweb, Irancell, TCI/DSL and Rightel, based on MahsaNet's published fragment guidance
- **First-run wizard** - language, servers, geo data: a four-step tour the first time the app starts
- Smaller: busy ports auto-shift before connecting, no flashing console windows, full English/Persian parity on every new screen

### New in v0.3.4

- **Connection tab** - one-click connect for system proxy / TUN / Psiphon-only / Psiphon-after with live status
- **Settings search** - find any setting by typing
- **Advanced-option collapse** - rarely used options hide behind expand/collapse toggles
- **Subscription URL import** - "Get link" and pasting a subscription URL now fetch configs directly; clipboard QR import button added
- **Failover mode** - round-robin server toggle in settings
- Assorted fixes: subscription dialog, connection-tab boxes, phantom error after imports

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

No analytics, no telemetry: nothing is ever sent to us. To do its job the app does contact a few third parties, and only these:

- Your proxy servers and your subscription URLs (always).
- GitHub (`api.github.com`, `github.com`) for the update check and for engine/geo downloads.
- When you connect: `www.gstatic.com` (connectivity check) and `api.ipify.org` (IP-leak check - once directly, once through the tunnel).
- On the Routing view and in the "What is my IP" tool: `ipwho.is` (IP/ISP lookup from your direct connection).
- When a Psiphon mode is active: the Psiphon network (that is the tunnel itself).
- When you run the speed test or the DNS scan: `speed.cloudflare.com` and the DNS servers listed in the scanner.
- When you run the IP scanner: TCP probes to the candidate IPs of the chosen preset (Cloudflare, Google, DigitalOcean, Akamai, Fastly, Bunny, Cloudflare WARP/MASQUE) or to your custom list.

Configuration lives in `%APPDATA%\com.mahsadesk.desktop`.

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