<div align="center">

# <img src="images/VRCX-0.png" alt="VRCX-0 Logo" width="25"> VRCX-0

### 更快、更輕的 VRCX。

[English](README.md) | [简体中文](README.zh-CN.md) | 繁體中文 | [日本語](README.ja-JP.md) | [한국어](README.ko-KR.md)

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

</div>

VRCX-0 是 VRCX 的完全重寫版本，由 VRCX 前任維護者之一開發，底層改為原生 Rust 核心（Tauri + React）。重寫帶來最直接的好處就是快：多年累積的歷史資料也能保持流暢，記憶體和安裝體積都比原版小得多。

首次啟動會自動匯入你現有的 VRCX 資料與設定，原始資料不會被更動，隨時可以換回去。

原版 VRCX 已轉向以維護為主，新功能都在 VRCX-0 開發。

## 安裝

在 [最新 Release](https://github.com/Map1en/VRCX-0/releases/latest) 下載對應平台的檔案：

| 平台                | 檔案                                       |
| ------------------- | ------------------------------------------ |
| Windows             | `VRCX-0_<版本號>_windows_x86_64_setup.exe` |
| macOS（Apple 晶片） | `VRCX-0_<版本號>_macos_aarch64.dmg`        |
| macOS（Intel）      | `VRCX-0_<版本號>_macos_x86_64.dmg`         |
| Linux               | `.AppImage`、`.deb` 或 `.rpm`              |

只需下載這一次 — 之後 VRCX-0 會自動更新。

## 主要特點

- **多年紀錄也不拖慢** — 在 VRCX 裡明顯變卡的資料量，放到 VRCX-0 依然流暢；老電腦、NAS 等級的小主機上也能流暢運作
- **記憶體用量比 VRCX 低約 50%–70%** — **背景模式**開啟後可降至僅數十 MB，所有核心功能照常運作
- **比一個模型還小** — 安裝程式 10 多 MB，安裝後 30 多 MB，比 VRCX 小 10 倍以上
- **遷移零負擔** — 自動匯入 VRCX 的資料庫與設定，原始資料不會被更動

其他特性：

- **社交 AI** — 內建助手，幫你讀懂自己的 VRChat 社交：問問最常和誰一起玩、正在和誰漸行漸遠，或什麼時候上線最容易遇到好友。接入你自己的 OpenAI 相容端點，也支援本地 LLM
- **MCP 伺服器** — 在本機執行一個附權杖保護的伺服器，把你的 VRCX-0 社交資料開放給 Claude 等 MCP 相容的 AI 用戶端，在你慣用的工具中直接查詢
- **每個帳號都有獨立的本機記錄** — 遊戲記錄等帳號相關資料會依目前登入的帳號分開儲存。使用多個帳號時，新的記錄不會再混在同一條時間軸；升級後仍可查看既有記錄
- **備份與還原** — 一鍵將資料打包成單一壓縮備份檔，也可設定定期備份並自動保留多個歷史版本；需要時可隨時從備份還原
- **分享世界收藏集** — 把收藏的世界整理成可分享的收藏集頁面。收到連結的人可以在瀏覽器中查看收藏集、前往 VRChat 官網開啟其中的世界，或將整個收藏集匯入 VRCX-0；也可以為單一世界和角色產生 VRCX-0 分享連結
- **社交自動化** — 依時間、實例類型或在場人員自動切換狀態與簽名；自動接受邀請請求；規則失效後自動還原原有狀態
- **輕量 VR 腕部 Overlay**，效能影響極低；同時支援 OpenVR（SteamVR）和 **OpenXR（Linux / WiVRn / Monado）**
- **社群主題** — 瀏覽並安裝主題商城中的主題，設定自訂背景圖片，還可疊加自己的 CSS
- **四通道通知系統** — 桌面通知、TTS 語音、VR Overlay 推播、Webhook，每個通道可依事件類型獨立設定
- **Webhook 通知** — 將事件轉發到任意 Webhook URL，採用 Discord 相容格式；可精確選擇要傳送的欄位
- 全介面支援完整鍵盤導航
- 無介面模式（Headless），適合進階用途 — 詳見 `crates/headless`

## 資料遷移

首次啟動時，VRCX-0 可自動匯入現有 VRCX 的資料庫與設定，原始資料不會被修改。舊使用者無需手動設定，即可繼續使用。

日後需要更換資料目錄時，只要選擇新位置，VRCX-0 就會自動移轉資料。

## 授權條款

本儲存庫的初始提交對應分叉時的上游 VRCX 快照，依 MIT License 發布。

fork 後新增、修改、重寫及新建的所有程式碼，均依 GNU General Public License v3.0（GPLv3）發布。

[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0?ref=badge_large)

## 從原始碼建置

僅在你想參與開發時才需要 — 詳見 [CONTRIBUTING.md](CONTRIBUTING.md)。

依賴：Node.js ≥ 24.10、npm ≥ 11.5，以及透過 rustup 安裝的穩定版 Rust 工具鏈。
Windows 使用者還需安裝 **Visual Studio Build Tools**，並勾選 **「使用 C++ 的桌面開發」** 工作負載。

```bash
git clone https://github.com/Map1en/VRCX-0
cd VRCX-0

npm install
npm run tauri:dev
```
