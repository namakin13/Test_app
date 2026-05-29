# Control Center

Windows向けのデスクトップアプリです。音声出力デバイスの切り替えと、Steamゲームライブラリの表示・起動をひとつの画面で行えます。

![Tauri](https://img.shields.io/badge/Tauri-2.x-blue)
![React](https://img.shields.io/badge/React-19-61DAFB)
![TypeScript](https://img.shields.io/badge/TypeScript-5.x-3178C6)
![Platform](https://img.shields.io/badge/Platform-Windows-0078D6)

## 機能

### Audio タブ
- PCに接続されているオーディオ出力デバイスを一覧表示
- クリック一つでデフォルト出力デバイスを切り替え

### Games タブ
「Steam」「その他」のサブタブで切り替えられます。

**Steam**
- Steamのインストール済みゲームライブラリを自動検出・一覧表示
- ゲームカードをクリックしてゲームを起動
- ヘッダー右上のSteamボタンからSteamアプリを起動

**その他**
- Steam以外のゲーム/アプリを手動で追加（「＋」カードからexe/lnk/bat/cmd/urlを選択）
- 追加したカードをクリックして起動、ホバー時の「×」ボタンで削除

**共通**
- ゲームカードに画像をドラッグ＆ドロップしてサムネイルをカスタマイズ

## 動作環境

- Windows 10 / 11 (x64)
- Steam がインストールされていること（Gamesタブ使用時）

## インストール

[Releases](https://github.com/namakin13/Test_app/releases) から最新の `.msi` ファイルをダウンロードして実行してください。

## 開発環境のセットアップ

### 必要なもの

- [Node.js](https://nodejs.org/) 18以上
- [Rust](https://www.rust-lang.org/) (stable)
- [Tauri CLI v2](https://tauri.app/)

### セットアップ

```bash
git clone https://github.com/namakin13/Test_app.git
cd Test_app
npm install
```

### 開発サーバー起動

```bash
npm run tauri dev
```

### ビルド

```bash
# MSIインストーラー
npm run tauri -- build --bundles msi

# すべてのフォーマット
npm run tauri build
```

ビルド成果物は `src-tauri/target/release/bundle/` に出力されます。

## 技術スタック

| 層 | 技術 |
|---|---|
| フロントエンド | React 19, TypeScript, Vite |
| バックエンド | Rust, Tauri 2 |
| UI | Framer Motion, Lucide React |
| OS連携 | Windows COM API, PowerShell, WinReg |

## ファイル構成

```
src/
├── App.tsx                  # ルートコンポーネント
├── components/
│   └── GameBanner.tsx       # ゲームサムネイル表示
├── hooks/
│   ├── useAudio.ts          # 音声デバイス状態管理
│   ├── useSteam.ts          # Steam関連状態管理
│   └── useManualGames.ts    # 手動追加ゲーム状態管理
└── index.css                # スタイル

src-tauri/src/
├── main.rs                  # エントリーポイント
├── audio_manager.rs         # 音声デバイス操作 (Windows COM API)
├── steam_manager.rs         # Steam連携 (ACFパース・起動)
├── manual_game_manager.rs   # 手動追加ゲーム管理 (登録・起動・削除)
└── icon_manager.rs          # カスタムアイコン管理
```
