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
  socks / http / hysteria2, plus base64 subscription payloads and clipboard import
- **QR tools** (the Android scanner, desktop style): decode a QR code from a
  clipboard screenshot, capture a screen region (Windows screen-clip) and import
  the config, and show any profile as a QR code to move it back to a phone
- **Test tools**: TCP ping, real URL latency test through a throwaway core instance
  (the v2rayNG test service, per-server and batched), and a config-validate tool
  that boots the core headlessly with the active profile
- **Per-app routing** (TUN): pick which running programs bypass the tunnel or are
  the only ones routed through it (sing-box process rules)
- **Subscriptions** - add URLs, manual and timed auto-update
- **TLS fragment** settings (packets / length / interval) wired into the Xray TLS dialer
- **CDN IP rewrite** - Manually replace the connect address with a Cloudflare IP while
  keeping SNI/Host (same idea as Mahsang's "config IP")
- **Routing presets** - proxy everything / bypass Iran (+China) with ad blocking, plus
  per-domain overrides; geoip/geosite data auto-download
- **Geo assets manager** - view, update and delete geoip.dat/geosite.dat
- **DNS settings** (remote DoH/UDP), sniffing, MUX, allow-insecure, IPv6 preference
- **System tray** - connect/disconnect, open, quit; minimize-to-tray; auto launch at login;
  auto-connect at startup
- **Live core logs** and session traffic counters
- **hysteria2 support** - parse hysteria2:// / hy2:// links; hysteria2 profiles run
  natively through the TUN engine (sing-box outbound)
- **Deep link import** - `mahsadesk://` URL scheme to hand configs to the app
- **Persian (RTL) and English** interface
- **Config export/import** of the whole pool (servers + subs + settings) as JSON

## New in v0.3

- **Psiphon only / Psiphon after** - the Psiphon tunnel engine (`psiphon-tunnel-core.exe`,
  same engine MahsaNG embeds on Android) runs as the standalone local tunnel in "only"
  mode, or chains on top of the active config in "after" mode (traffic: app → Psiphon →
  your config). Egress country, protocol family (normal / conduit-inproxy), aggressive
  mode and local ports are configurable; an optional user base config is read from
  `psiphon-user.json` in the app data folder. Drop `psiphon-tunnel-core.exe` from the
  Psiphon-Labs builds next to `xray.exe`.
- **DNS scanner** (Tools) - queries the same domain through the system resolver, Google,
  Cloudflare, Quad9, Shecan, 403.online, Radar, Electro, Begzar and two DoH references;
  shows latency and flags answers that differ from the DoH truth (filtering detection)
- **Mahsa IP scanner** (Tools) - TCP-probes candidate CDN IPs on the chosen port from
  preset Cloudflare/Google/DigitalOcean blocks or your own IP/CIDR list, and one click
  pushes the fastest working IP into the CDN address-rewrite setting
- **Network tools** (Tools) - ping, traceroute, DNS lookup, TCP port check, "what is my
  IP" (with geo/ISP info) and a download speed test
- **Encrypted Client Hello (ECH)** - switch in settings; injects
  `streamSettings.tlsSettings.echConfigList` for the Xray core (auto via DoH when empty)
- **Domain fronting (fake host)** - connects to a fronting CDN domain while the real
  destination stays in the Host header and the CDN is presented as SNI

## New in v0.3.2

- **Startup update check** - on every launch the app silently compares its version
  against the GitHub release feed and, when a newer MahsaDesk exists, shows a banner
  with a direct installer download link (EN + FA)
- **Green/red switch colors** - all on/off switches and the connection status dot now
  use fixed brand colors (#01ca7a on / #f03b4c off) in both themes

## New in v0.3.1

- **Light / dark / auto theme** selection in settings; `auto` follows the OS color scheme
- **Requirements manager** - a settings card checks every engine requirement (Xray,
  sing-box, wintun, geo data, Psiphon engine) against upstream, shows versions, and
  upgrades them in place from inside the app
- **Complete builds** - the installer and portable zip now bundle every runtime
  requirement, including `psiphon-tunnel-core.exe` (official Psiphon-Labs binary), so
  Psiphon works with zero manual steps

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
