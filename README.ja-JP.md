<div align="center">

# <img src="images/VRCX-0.png" alt="VRCX-0 Logo" width="25"> VRCX-0

### もっと速く、もっと軽い VRCX。

[English](README.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-Hant.md) | 日本語 | [한국어](README.ko-KR.md)

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

VRCX-0 は、以前 VRCX のメンテナーを務めていたメンバーの一人が一から書き直したバージョンです。ネイティブな Rust コア（Tauri + React）で再構築されており、パフォーマンスが大きく向上しています。何年分の記録が積み重なっても動作は軽いまま、メモリ使用量もインストールサイズも VRCX を大きく下回ります。

初回起動時に、既存の VRCX のデータと設定を自動で引き継ぎます。元のデータには一切手を加えないため、いつでも元の環境に戻れます。

本家 VRCX はメンテナンス中心に移行しており、VRCX-0 では引き続き新機能を開発しています。

## インストール

[最新リリース](https://github.com/Map1en/VRCX-0/releases/latest) から、お使いのプラットフォーム向けのファイルをダウンロードしてください。

| プラットフォーム        | ファイル                                       |
| ----------------------- | ---------------------------------------------- |
| Windows                 | `VRCX-0_<バージョン>_windows_x86_64_setup.exe` |
| macOS（Apple シリコン） | `VRCX-0_<バージョン>_macos_aarch64.dmg`        |
| macOS（Intel）          | `VRCX-0_<バージョン>_macos_x86_64.dmg`         |
| Linux                   | `.AppImage`、`.deb`、`.rpm`                    |

ダウンロードは最初の一度だけ。以降は VRCX-0 が自動で更新します。

## 主な特徴

- **何年遊んでも重くならない** — VRCX では目に見えて重くなるデータ量でも、VRCX-0 なら軽快に動作。低スペック PC や NAS クラスの小型 PC でも問題なく動きます
- **メモリ使用量は VRCX 比で約 50%〜70% 減** — **バックグラウンドモード**をオンにすると数十 MB まで下がり、すべてのコア機能はそのまま動き続けます
- **アバター 1 体分より小さい** — インストーラーは 10 MB 台、インストール後も 30 MB 台。VRCX の 10 分の 1 以下のサイズです
- **乗り換えの手間はほぼゼロ** — VRCX のデータベースと設定を自動でインポート。元のデータは一切変更されません

このほかにも：

- **AI アシスタント** — VRChat 生活をふり返るための内蔵アシスタント。よく一緒に遊ぶ相手、疎遠になりつつある相手、フレンドをつかまえやすい時間帯などを質問でき、自分の AI サービスをつなぐだけで使えます
- **MCP サーバー** — 外部の AI ツールからローカルのソーシャルデータに直接アクセス可能。内蔵アシスタントよりはるかに柔軟で、上級者におすすめです
- **アカウント別のローカル履歴** — ゲームログなどの履歴はアカウントごとに分けて保存され、複数アカウントでも混ざりません
- **バックアップと復元** — ワンクリックで圧縮バックアップ、定期自動バックアップにも対応し、何世代分でも残せます。いつでも復元できます
- **ワールドコレクションの共有** — お気に入りのワールドを共有ページにまとめ、相手は見る・開く・インポートができます。ワールドやアバター単体の共有リンクも作成できます
- **ソーシャルオートメーション** — 時間帯・インスタンスの種類・一緒にいる相手に応じてステータスや自己紹介を自動変更；招待リクエストの自動承認；ルール終了後に元の状態へ自動復元
- **軽量な VR 手首 Overlay**、パフォーマンスへの影響は最小限；OpenVR（SteamVR）と **OpenXR（Linux / WiVRn / Monado）** の両方に対応
- **コミュニティテーマ** — カタログからテーマを閲覧してインストール、カスタム背景画像の設定、さらに独自の CSS を重ねがけ可能
- **通知** — デスクトップ・読み上げ（TTS）・VR Overlay・Webhook の 4 チャンネルをイベントごとに個別に設定できます；Webhook は Discord 互換フォーマットに対応
- アプリ全体で完全なキーボードナビゲーションに対応
- 上級者向けのヘッドレスモードも搭載 — `crates/headless` を参照

## ライセンス

このリポジトリの初回コミットは、フォーク時点の上流 VRCX スナップショットに対応しており、MIT License に従います。

フォーク後に追加・変更・書き直されたすべてのコードは、GNU General Public License v3.0（GPLv3）に従います。

[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2FMap1en%2FVRCX-0?ref=badge_large)

## ソースからビルド

開発に参加する場合のみ必要です。詳しくは [CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。

必要なもの：Node.js ≥ 24.10、npm ≥ 11.5、rustup 経由でインストールした安定版 Rust ツールチェーン。
Windows の場合は、**Visual Studio Build Tools** をインストールし、**「C++ によるデスクトップ開発」** ワークロードを選択してください。

```bash
git clone https://github.com/Map1en/VRCX-0
cd VRCX-0

npm install
npm run tauri:dev
```
