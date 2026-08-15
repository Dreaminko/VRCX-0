<div align="center">

# <img src="images/VRCX-0.png" alt="VRCX-0 Logo" width="25"> VRCX-0

### The fast, lightweight VRCX.

English | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-Hant.md) | [日本語](README.ja-JP.md) | [한국어](README.ko-KR.md)

[![Release](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/Map1en/VRCX-0/badge-data/version.json&style=flat&color=4340a2&labelColor=1f2328&logo=github&logoColor=white)](https://github.com/Map1en/VRCX-0/releases/latest)
[![Downloads](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/Map1en/VRCX-0/badge-data/downloads.json&style=flat&color=4340a2&labelColor=1f2328)](https://github.com/Map1en/VRCX-0/releases)
[![Installer](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/Map1en/VRCX-0/badge-data/windows-installer-size.json&style=flat&label=installer&color=4340a2&labelColor=1f2328&logo=windows&logoColor=white)](https://github.com/Map1en/VRCX-0/releases/latest)
[![Discord](https://img.shields.io/discord/1494343220467994644?style=flat&logo=discord&logoColor=white&label=discord&color=5865f2&labelColor=1f2328)](https://discord.gg/fehKP3SVPN)
<br>
[![CI](https://img.shields.io/github/actions/workflow/status/Map1en/VRCX-0/ci.yml?branch=master&label=CI&style=flat&labelColor=1f2328)](https://github.com/Map1en/VRCX-0/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/Map1en/VRCX-0/badge-data/coverage.json&style=flat&color=brightgreen&labelColor=1f2328)](https://github.com/Map1en/VRCX-0/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-GPL--3.0%20%2B%20MIT-4c566a?style=flat&labelColor=1f2328)](LICENSE)
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0?ref=badge_shield)

[![Download](https://img.shields.io/badge/Download%20VRCX--0-4340a2?style=for-the-badge)](https://github.com/Map1en/VRCX-0/releases/latest)

Windows · macOS · Linux

![VRCX-0](images/screenshot-user-dialog.webp)

</div>

VRCX-0 is a ground-up rewrite of VRCX by one of its former maintainers, rebuilt on a native Rust core (Tauri + React) with significantly improved performance: years of accumulated history stay smooth, and both memory usage and install size are far below the original.

On first launch it automatically imports your existing VRCX data and settings. The original data is never modified — you can switch back at any time.

The upstream VRCX project has shifted toward maintenance; VRCX-0 is where new features are being built.

## Install

Grab the file for your platform from the [latest release](https://github.com/Map1en/VRCX-0/releases/latest):

| Platform              | File                                        |
| --------------------- | ------------------------------------------- |
| Windows               | `VRCX-0_<version>_windows_x86_64_setup.exe` |
| macOS (Apple Silicon) | `VRCX-0_<version>_macos_aarch64.dmg`        |
| macOS (Intel)         | `VRCX-0_<version>_macos_x86_64.dmg`         |
| Linux                 | `.AppImage`, `.deb`, or `.rpm`              |

You only need to do this once — VRCX-0 updates itself from then on.

## Highlights

- **Years of history won't slow it down** — data that makes VRCX visibly
  sluggish stays smooth in VRCX-0; it runs fine even on a potato PC or a home
  server
- **About 50%–70% less memory than VRCX** in normal use
- **Background mode** brings it down to just tens of MB while all core
  features keep running
- **Smaller than a single avatar bundle** — just over 10 MB to download, just
  over 30 MB on disk; over 10× smaller than VRCX
- **Zero-friction migration** — your VRCX database and settings import
  automatically; the original data is never modified

Beyond that:

- **Social AI** — a built-in assistant that helps you make sense of your VRChat
  life: ask who you play with most, who you're drifting away from, or the best
  time to catch friends online. Connect your own AI service to get started
- **MCP server** — let external AI tools access your local social data directly,
  far more flexible than the built-in assistant; recommended for advanced users
- **Per-account local history** — game logs and account-specific history are
  stored separately, so activity no longer gets mixed into a single timeline
  when you use multiple accounts
- **Backup & restore** — one-click compressed backup with scheduled automatic
  backups and multiple versions; restore from any backup at any time
- **Shareable world collections** — turn your favorite worlds into a shareable
  page others can browse, open, or import; also supports share links for
  individual worlds and avatars
- **Social Automation** — auto-switch your status and bio based on time of day,
  instance type, or who you're with; auto-accept invite requests; restores your
  previous state when rules expire
- **Lightweight VR wrist overlay** with minimal performance impact; supports both
  OpenVR (SteamVR) and **OpenXR (Linux / WiVRn / Monado)**
- **Community Themes** — browse and install themes from a catalog, set a custom
  background image, and layer your own CSS on top
- **Notifications** — desktop, text-to-speech, VR overlay, and webhooks — four
  channels independently configured per event type; webhooks use a
  Discord-compatible format
- Full keyboard navigation
- Headless mode for advanced setups — see `crates/headless`

## License

The initial commit of this repository corresponds to the upstream VRCX snapshot at the time of the fork and is licensed under the MIT License.

All modifications, additions, rewrites, and new code introduced after the fork are licensed under the GNU General Public License v3.0 (GPLv3).

[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0?ref=badge_large)

## Building from source

Only needed if you want to contribute — see [CONTRIBUTING.md](CONTRIBUTING.md).

Requirements: Node.js ≥ 24.10, npm ≥ 11.5, and a stable Rust toolchain via rustup.
On Windows, also install **Visual Studio Build Tools** with the **Desktop development with C++** workload.

```bash
git clone https://github.com/Map1en/VRCX-0
cd VRCX-0

npm install
npm run tauri:dev
```
