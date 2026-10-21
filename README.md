# MahsaNG

**A free VPN for everyone, powered by its community.**

MahsaNG is a VPN app for Android and Windows. Instead of selling servers, the community shares unused server bandwidth — the app finds and connects you to the fastest working one automatically. No account, no payment.

<img src="https://github.com/GFW-knocker/MahsaNG/blob/master/Mahsa_logo.jpg?raw=true" width="360" alt="MahsaNG logo" >

## Download

| Platform | Where |
| --- | --- |
| Android (Google Play) | [MahsaNG on Play Store](https://play.google.com/store/apps/details?id=com.MahsaNet.MahsaNG) |
| Android (APK) | [Download latest release](https://github.com/GFW-knocker/MahsaNG/releases/latest) |
| Windows (desktop app) | [MahsaDesk releases](https://github.com/pourjahani/MahsaDesk/releases) |
| Telegram channel | [t.me/mahsa_net](https://t.me/mahsa_net) |

دانلود اندروید: [آخرین نسخه از گیتهاب](https://github.com/GFW-knocker/MahsaNG/releases/latest) · [کانال تلگرام](https://t.me/mahsa_net)

## Why MahsaNG?

- **Free** — no subscription, no account.
- **Community-powered** — people share their unused VPS bandwidth so others can get online.
- **Fair** — shared configs are protected so they can't be stolen and resold.
- **Hard to block** — built-in tools like DNS over HTTPS, TLS fragment, fake host, rotating configs and random subdomains.

## How it works

MahsaNG is the user app of the [Segaro_Dream](https://github.com/GFW-knocker/Segaro_Dream) project. Community members donate server configs; the system watches which ones still work for each network (ISP) and automatically hands the best ones to users. The more people use and rate configs, the smarter the selection gets.

Our goal: a VPN that is **free, stable, scalable, distributed and filter-resistant**.

## What's in this repo?

| Folder | What it is |
| --- | --- |
| `V2rayNG/` | The Android app source |
| `MahsaDesk/` | The Windows desktop app ([MahsaDesk](https://github.com/pourjahani/MahsaDesk)) |
| `AndroidLibV2rayLite/` | The native networking library used by the app |

## Building from source

**The Android app cannot be built into the Play Store APK directly from this repo.** Two parts are closed on purpose:

1. Config encryption — keeps donated configs from being copied and resold.
2. Server authentication — keeps the Mahsa server from being abused.

Everything used to bypass filtering is open: it is published in [Mahsa Xray-Core](https://github.com/GFW-knocker/Xray-core), including Wireguard Noise, YouTube Direct, TLS/HTTPS/DoH fragmenting and fake host.

**Want a fully open build?** Use [NikaNG](https://github.com/mahsanet/NikaNG) — a fully open-source fork of v2rayNG with the Mahsa Xray-core. It builds with GitHub Actions or locally. Before compiling, place [libv2ray.aar](https://github.com/GFW-knocker/AndroidLibXrayLite) into `v2rayng/apps/libs`.

## Community

Share your unused bandwidth, report what works, and help others stay online:

- Telegram: [t.me/mahsa_net](https://t.me/mahsa_net)
- Issues & ideas: [open an issue](https://github.com/GFW-knocker/MahsaNG/issues)
