# VideoPlayer → Test_app 機能移植 引き継ぎプロンプト

## 作業概要

VideoPlayer（Tauri + React + TypeScript）の2つの機能を、Test_app（同じスタック）に移植する。

---

## リポジトリ

- **移植元**: https://github.com/namakin13/VideoPlayer
- **移植先**: https://github.com/namakin13/Test_app

---

## 移植する機能

### ① メディアキー送信
- 再生/一時停止（`media_play_pause`）
- 次のトラック（`media_next`）
- 前のトラック（`media_prev`）
- Windows API `SendInput` でキー入力をシミュレート

### ② ブラウザ個別ミュート
- Chrome / Edge / Firefox 等を対象に `ISimpleAudioVolume` でミュート切り替え
- `sysinfo` でプロセス名を特定し、対象ブラウザのオーディオセッションを操作

---

## 移植先 Test_app の現状

### フロントエンド（src/）
- `App.tsx` — タブUI（Audio / Games）
- `hooks/useAudio.ts` — オーディオデバイス切り替え
- `hooks/useSteam.ts` — Steamゲームランチャー
- `components/GameBanner.tsx` — ゲームバナー表示

### バックエンド（src-tauri/src/）
- `main.rs` — エントリポイント、コマンド登録
- `audio_manager.rs` — オーディオデバイス列挙・切り替え（`get_audio_devices`, `switch_audio_device`）
- `steam_manager.rs` — Steam連携
- `icon_manager.rs` — カスタムアイコン管理

---

## 必要な変更

### 1. `src-tauri/Cargo.toml`

`[dependencies]` に `sysinfo` を追加：

```toml
sysinfo = "0.38.4"
```

`windows` の `features` に以下を追加：

```toml
"Win32_UI_Input_KeyboardAndMouse",
"Win32_System_Threading",
"Win32_System_ProcessStatus",
```

### 2. `src-tauri/src/audio_manager.rs`

ファイル末尾に以下を追記：

```rust
// ===== メディアキー送信 =====
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, KEYBDINPUT, VIRTUAL_KEY,
    KEYBD_EVENT_FLAGS, INPUT_0,
};

fn send_media_key(vk: u16) {
    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}

#[tauri::command]
pub fn media_play_pause() {
    send_media_key(0xB3);
}

#[tauri::command]
pub fn media_next() {
    send_media_key(0xB0);
}

#[tauri::command]
pub fn media_prev() {
    send_media_key(0xB1);
}

// ===== ブラウザ個別ミュート =====
use sysinfo::System;
use windows::Win32::Media::Audio::{
    IAudioSessionControl2, IAudioSessionEnumerator,
    IAudioSessionManager2, ISimpleAudioVolume,
};

#[tauri::command]
pub fn toggle_browser_mute() -> std::result::Result<bool, String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let result = (|| -> windows::core::Result<bool> {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let session_manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
            let session_enumerator: IAudioSessionEnumerator =
                session_manager.GetSessionEnumerator()?;
            let count = session_enumerator.GetCount()?;

            let mut system = System::new_all();
            let browsers = ["chrome", "msedge", "firefox", "opera", "brave", "vivaldi"];

            let mut toggled = false;
            let mut final_state = false;

            for i in 0..count {
                if let Ok(session) = session_enumerator.GetSession(i) {
                    if let Ok(session2) = session.cast::<IAudioSessionControl2>() {
                        if let Ok(pid) = session2.GetProcessId() {
                            if pid > 0 {
                                if let Some(process) =
                                    system.process(sysinfo::Pid::from_u32(pid))
                                {
                                    let name =
                                        process.name().to_string_lossy().to_lowercase();
                                    if browsers.iter().any(|b| name.contains(b)) {
                                        if let Ok(volume) =
                                            session.cast::<ISimpleAudioVolume>()
                                        {
                                            if !toggled {
                                                if let Ok(current_mute) = volume.GetMute() {
                                                    final_state = !current_mute.as_bool();
                                                    toggled = true;
                                                }
                                            }
                                            let _ = volume.SetMute(final_state, std::ptr::null());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(final_state)
        })();

        match result {
            Ok(state) => Ok(state),
            Err(e) => Err(e.to_string()),
        }
    }
}
```

> ⚠️ `audio_manager.rs` はワイルドカード `use windows::Win32::Media::Audio::*` を使用済み。
> `IAudioSessionControl2` 等が重複する場合は追記側の個別importを削除してコンパイル確認すること。

### 3. `src-tauri/src/main.rs`

useに追加：

```rust
use audio_manager::{
    get_audio_devices, switch_audio_device,
    media_play_pause, media_next, media_prev, toggle_browser_mute,
};
```

`invoke_handler` に追加：

```rust
.invoke_handler(tauri::generate_handler![
    get_audio_devices,
    switch_audio_device,
    media_play_pause,    // 追加
    media_next,          // 追加
    media_prev,          // 追加
    toggle_browser_mute, // 追加
    get_steam_games,
    launch_steam_game,
    launch_steam_app,
    get_custom_icons,
    save_custom_icon,
    save_custom_icon_from_path,
    get_local_steam_image
])
```

### 4. `src/App.tsx`

importに追加：

```tsx
import { invoke } from "@tauri-apps/api/core";
import { SkipBack, Play, SkipForward, VolumeX } from "lucide-react";
```

`App()` 関数内のstateに追加：

```tsx
const [browserMuted, setBrowserMuted] = React.useState(false);
```

コンポーネント外にスタイル定数を追加：

```tsx
const mediaBtnStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  width: "44px",
  height: "44px",
  background: "rgba(255,255,255,0.05)",
  border: "1px solid rgba(255,255,255,0.1)",
  borderRadius: "12px",
  color: "white",
  cursor: "pointer",
  transition: "all 0.2s ease",
};
```

Audioタブ（`<motion.div key="audio-tab">` 内）の `device-list` の直前にUIを追加：

```tsx
{/* メディアコントロール */}
<div style={{
  display: "flex",
  gap: "8px",
  justifyContent: "center",
  marginBottom: "16px",
  padding: "12px",
  background: "rgba(255,255,255,0.03)",
  borderRadius: "16px",
  border: "1px solid rgba(255,255,255,0.08)",
}}>
  <button onClick={() => invoke("media_prev")} title="前へ" style={mediaBtnStyle}>
    <SkipBack size={18} />
  </button>
  <button onClick={() => invoke("media_play_pause")} title="再生/一時停止" style={mediaBtnStyle}>
    <Play size={18} />
  </button>
  <button onClick={() => invoke("media_next")} title="次へ" style={mediaBtnStyle}>
    <SkipForward size={18} />
  </button>
  <button
    onClick={async () => {
      const newMuted: boolean = await invoke("toggle_browser_mute");
      setBrowserMuted(newMuted);
    }}
    title="ブラウザミュート"
    style={{
      ...mediaBtnStyle,
      background: browserMuted ? "rgba(255,80,80,0.3)" : "rgba(255,255,255,0.05)",
      borderColor: browserMuted ? "rgba(255,80,80,0.5)" : "rgba(255,255,255,0.1)",
    }}
  >
    <VolumeX size={18} />
  </button>
</div>
```

---

## 作業手順

1. Test_appリポジトリをクローン／開く
2. 上記4ファイルを順番に修正
3. `cargo build` でコンパイル確認（Rustエラーを先に潰す）
4. `npm run tauri dev` で動作確認
5. コンパイルエラーが出た場合は `audio_manager.rs` のimport重複を確認

---

## 注意事項

- Windows専用コード（Windows API依存）のため、ビルドはWindowsマシンで行うこと
- `sysinfo` の `process.name()` は `OsStr` を返すため、`.to_string_lossy().to_lowercase()` で処理すること（`.to_lowercase()` 直呼び出し不可）
- `eMultimedia` は `eRender` と同じ `windows::Win32::Media::Audio` に属する。ワイルドカードimport済みなら追加不要
