<div align="center">

# <img src="images/VRCX-0.png" alt="VRCX-0 Logo" width="25"> VRCX-0

### 更快、更轻的 VRCX。

[English](README.md) | 简体中文 | [繁體中文](README.zh-Hant.md) | [日本語](README.ja-JP.md) | [한국어](README.ko-KR.md)

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

VRCX-0 是 VRCX 的完全重写版本，由 VRCX 前维护者之一开发，底层采用原生 Rust 核心（Tauri + React）。重写带来最直接的好处就是快：多年积累的数据也能保持流畅，内存和安装体积都比原版小得多。

首次启动会自动导入你现有的 VRCX 数据和设置，原始数据不会被改动，随时可以换回去。

原版 VRCX 已转向以维护为主，新功能都在 VRCX-0 开发。

## 安装

在 [最新 Release](https://github.com/Map1en/VRCX-0/releases/latest) 里下载对应平台的文件：

| 平台                | 文件                                       |
| ------------------- | ------------------------------------------ |
| Windows             | `VRCX-0_<版本号>_windows_x86_64_setup.exe` |
| macOS（Apple 芯片） | `VRCX-0_<版本号>_macos_aarch64.dmg`        |
| macOS（Intel）      | `VRCX-0_<版本号>_macos_x86_64.dmg`         |
| Linux               | `.AppImage`、`.deb` 或 `.rpm`              |

只需下载这一次 — 之后 VRCX-0 会自动更新。

## 主要特点

- **多年记录也不拖慢** — 在 VRCX 里明显变卡的数据量，放到 VRCX-0 依然流畅；土豆机、NAS 级别的小主机上也能流畅运行
- **内存占用比 VRCX 低约 50%–70%** — **后台模式**开启后可降至仅几十 MB，所有核心功能照常运行
- **比一个模型包还小** — 安装包 10 多 MB，安装后 30 多 MB，比 VRCX 小 10 倍以上
- **迁移零负担** — 自动导入 VRCX 的数据库和设置，原始数据不会被改动

其他特性：

- **AI 助手** — 内置助手，帮你读懂自己的 VRChat 社交：问问最常和谁一起玩、正在和谁渐行渐远，或什么时候上线最容易遇到好友。接入你自己的 OpenAI 兼容端点，也支持本地 LLM
- **MCP 服务器** — 在本地运行一个带令牌保护的服务器，把你的 VRCX-0 社交数据开放给 Claude 等 MCP 兼容的 AI 客户端，在你常用的工具中直接查询
- **每个账号都有独立的本地历史** — 游戏日志等账号相关记录会按当前登录账号分别保存，多账号使用时，新记录不会再混进同一条时间线；升级后仍可查看既有历史
- **备份与恢复** — 一键将数据打包成单个压缩备份文件，也可设置定时备份并自动保留多个历史版本；需要时可随时从备份恢复
- **分享世界合集** — 把收藏的世界整理成可分享的合集页面。收到链接的人可以在浏览器中查看合集、前往 VRChat 官网打开其中的世界，或将整个合集导入 VRCX-0；也可以为单个世界和模型生成 VRCX-0 分享链接
- **社交自动化** — 按时间、实例类型或在场人员自动切换状态和签名；自动接受邀请请求；规则失效后自动恢复原有状态
- **轻量 VR 腕部 Overlay**，性能影响极低；同时支持 OpenVR（SteamVR）和 **OpenXR（Linux / WiVRn / Monado）**
- **社区主题** — 浏览并安装主题商城中的主题，设置自定义背景图片，还可叠加自己的 CSS
- **四通道通知系统** — 桌面通知、TTS 语音、VR Overlay 推送、Webhook，每个通道按事件类型独立配置
- **Webhook 通知** — 将事件转发到任意 Webhook URL，采用 Discord 兼容格式；可精确选择要发送的字段
- 全界面支持完整键盘导航
- 无头模式（Headless），适合进阶用途 — 详见 `crates/headless`

## 数据迁移

首次启动时，VRCX-0 可自动导入现有 VRCX 的数据库和配置，原始数据不会被修改。老用户无需手动设置，即可继续使用。

以后需要更换数据目录时，只需选择新位置，VRCX-0 就会自动迁移数据。

## 许可

本仓库的第一个提交对应 fork 时的上游 VRCX 项目快照，遵循 MIT License。

fork 之后新增、修改、重写的所有代码，均遵循 GNU General Public License v3.0（GPLv3）。

[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0?ref=badge_large)

## 从源码构建

仅在你想参与开发时才需要 — 详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

依赖：Node.js ≥ 24.10、npm ≥ 11.5，以及通过 rustup 安装的稳定版 Rust 工具链。
Windows 用户还需安装 **Visual Studio Build Tools**，并勾选 **"使用 C++ 的桌面开发"** 工作负载。

```bash
git clone https://github.com/Map1en/VRCX-0
cd VRCX-0

npm install
npm run tauri:dev
```
