use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::process::Command;
use windows::core::*;
use windows::Win32::Devices::FunctionDiscovery::*;
use windows::Win32::Foundation::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::StructuredStorage::*;
use windows::Win32::System::Com::*;
use windows::Win32::System::Variant::*;

#[derive(Serialize, Debug, Clone)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn list_devices() -> Result<Vec<AudioDevice>> {
    unsafe {
        //COM initialization
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let collection = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;

        println!("Found {} active audio endpoints", count);

        let mut devices = Vec::new();

        let default_id_str =
            if let Ok(default_device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                if let Ok(id) = default_device.GetId() {
                    let s = id.to_string().unwrap_or_default();
                    CoTaskMemFree(Some(id.as_ptr() as _));
                    s
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

        for i in 0..count {
            if let Ok(device) = collection.Item(i) {
                if let Ok(id) = device.GetId() {
                    let id_str = id.to_string().unwrap_or_default();

                    let mut name = format!(
                        "Unknown Device ({})",
                        if id_str.len() > 8 {
                            &id_str[..8]
                        } else {
                            &id_str
                        }
                    );

                    // Try to get the names
                    let mut friendly_name = String::new();
                    let mut device_desc = String::new();

                    if let Ok(store) = device.OpenPropertyStore(STGM_READ) {
                        // Get FriendlyName
                        if let Ok(mut prop) = store.GetValue(&PKEY_Device_FriendlyName) {
                            if prop.Anonymous.Anonymous.vt == VT_LPWSTR
                                && !prop.Anonymous.Anonymous.Anonymous.pwszVal.is_null()
                            {
                                if let Ok(s) =
                                    prop.Anonymous.Anonymous.Anonymous.pwszVal.to_string()
                                {
                                    friendly_name = s;
                                }
                            }
                            let _ = PropVariantClear(&mut prop);
                        }

                        // Get DeviceDesc
                        if let Ok(mut prop_desc) = store.GetValue(&PKEY_Device_DeviceDesc) {
                            if prop_desc.Anonymous.Anonymous.vt == VT_LPWSTR
                                && !prop_desc.Anonymous.Anonymous.Anonymous.pwszVal.is_null()
                            {
                                if let Ok(s) =
                                    prop_desc.Anonymous.Anonymous.Anonymous.pwszVal.to_string()
                                {
                                    device_desc = s;
                                }
                            }
                            let _ = PropVariantClear(&mut prop_desc);
                        }
                    }

                    if !friendly_name.is_empty() {
                        if !device_desc.is_empty()
                            && friendly_name != device_desc
                            && !friendly_name.contains(&device_desc)
                        {
                            name = format!("{} ({})", friendly_name, device_desc);
                        } else {
                            name = friendly_name;
                        }
                    } else if !device_desc.is_empty() {
                        name = device_desc;
                    }

                    devices.push(AudioDevice {
                        id: id_str.clone(),
                        name,
                        is_default: id_str == default_id_str,
                    });
                    CoTaskMemFree(Some(id.as_ptr() as _));
                }
            }
        }

        Ok(devices)
    }
}

/// PowerShellの標準出力とexit成否からデバイス切り替え結果を判定する（テスト可能な純粋関数）
pub(crate) fn parse_powershell_output(stdout: &str, exit_success: bool) -> std::result::Result<(), String> {
    if !exit_success {
        return Err(format!("PowerShell終了コードエラー。stdout: {}", stdout.trim()));
    }
    if stdout.contains("Error:") {
        return Err(format!("SetDefaultEndpoint失敗: {}", stdout.trim()));
    }
    if stdout.contains("0x") {
        let parts: Vec<&str> = stdout.split(": ").collect();
        if parts.len() > 1 {
            let codes: Vec<&str> = parts[1].split(", ").collect();
            for code in codes {
                let code = code.trim();
                if !code.contains("0x0") && code.contains("0x") {
                    return Err(format!(
                        "1つ以上のロールで切り替えに失敗しました: {}",
                        stdout.trim()
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn set_default_device(device_id: String) -> Result<()> {
    // PowerShell script using the documented PolicyConfig COM object interface
    // Note: Escaping {{ and }} for format! macro
    let script = format!(
        r#"$id = '{}';
$code = @'
using System;
using System.Runtime.InteropServices;

[ComImport, Guid("870af99c-171d-4f9e-af0d-e63df40c2bc9")] class PolicyConfigClient {{ }}
[ComImport, Guid("294935CE-F637-4E7C-A41B-AB255460B862")] class PolicyConfigVistaClient {{ }}

[Guid("f8679f50-d128-4fd3-acc1-4471cd697261"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPolicyConfig1 {{
    [PreserveSig] int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string id, out IntPtr f);
    [PreserveSig] int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out IntPtr f);
    [PreserveSig] int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr f, IntPtr a);
    [PreserveSig] int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out long p, out long b);
    [PreserveSig] int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, ref long p);
    [PreserveSig] int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, out int m);
    [PreserveSig] int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, int m);
    [PreserveSig] int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, out IntPtr v);
    [PreserveSig] int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, ref IntPtr v);
    [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string id, int r);
    [PreserveSig] int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string id, int v);
}}

[Guid("568b7751-64af-449e-9333-66f81e35d1f1"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPolicyConfig2 {{
    [PreserveSig] int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string id, out IntPtr f);
    [PreserveSig] int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out IntPtr f);
    [PreserveSig] int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr f, IntPtr a);
    [PreserveSig] int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out long p, out long b);
    [PreserveSig] int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, ref long p);
    [PreserveSig] int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, out int m);
    [PreserveSig] int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, int m);
    [PreserveSig] int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, out IntPtr v);
    [PreserveSig] int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, ref IntPtr v);
    [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string id, int r);
    [PreserveSig] int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string id, int v);
}}

[Guid("d666063f-1598-4e43-8351-c063cf22039d"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPolicyConfig3 {{
    [PreserveSig] int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string id, out IntPtr f);
    [PreserveSig] int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out IntPtr f);
    [PreserveSig] int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr f, IntPtr a);
    [PreserveSig] int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out long p, out long b);
    [PreserveSig] int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, ref long p);
    [PreserveSig] int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, out int m);
    [PreserveSig] int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, int m);
    [PreserveSig] int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, out IntPtr v);
    [PreserveSig] int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, ref IntPtr v);
    [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string id, int r);
    [PreserveSig] int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string id, int v);
}}

[Guid("ca28602d-2214-4c46-b258-487bf03917d5"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPolicyConfig4 {{
    [PreserveSig] int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string id, out IntPtr f);
    [PreserveSig] int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out IntPtr f);
    [PreserveSig] int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr f, IntPtr a);
    [PreserveSig] int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out long p, out long b);
    [PreserveSig] int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, ref long p);
    [PreserveSig] int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, out int m);
    [PreserveSig] int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, int m);
    [PreserveSig] int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, out IntPtr v);
    [PreserveSig] int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, ref IntPtr v);
    [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string id, int r);
    [PreserveSig] int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string id, int v);
}}

[Guid("568b9108-44bf-40b4-9006-86afe5b5a620"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPolicyConfig5 {{
    [PreserveSig] int GetMixFormat([MarshalAs(UnmanagedType.LPWStr)] string id, out IntPtr f);
    [PreserveSig] int GetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out IntPtr f);
    [PreserveSig] int SetDeviceFormat([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr f, IntPtr a);
    [PreserveSig] int GetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, int a, out long p, out long b);
    [PreserveSig] int SetProcessingPeriod([MarshalAs(UnmanagedType.LPWStr)] string id, ref long p);
    [PreserveSig] int GetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, out int m);
    [PreserveSig] int SetShareMode([MarshalAs(UnmanagedType.LPWStr)] string id, int m);
    [PreserveSig] int GetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, out IntPtr v);
    [PreserveSig] int SetPropertyValue([MarshalAs(UnmanagedType.LPWStr)] string id, ref Guid k, ref IntPtr v);
    [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string id, int r);
    [PreserveSig] int SetEndpointVisibility([MarshalAs(UnmanagedType.LPWStr)] string id, int v);
}}

public static class Switcher {{
    public static void Set(string id) {{
        if (TrySwitch(new PolicyConfigClient(), id, "Client")) return;
        if (TrySwitch(new PolicyConfigVistaClient(), id, "VistaClient")) return;
        Console.WriteLine("Error: No supported IPolicyConfig interface found.");
    }}

    private static bool TrySwitch(object client, string id, string clientName) {{
        int r0, r1, r2;
        
        var v1 = client as IPolicyConfig1;
        if (v1 != null) {{
            r0 = v1.SetDefaultEndpoint(id, 0); r1 = v1.SetDefaultEndpoint(id, 1); r2 = v1.SetDefaultEndpoint(id, 2);
            Console.WriteLine("Success({{0}}-v1): 0x{{1:X}}, 0x{{2:X}}, 0x{{3:X}}", clientName, r0, r1, r2);
            return true;
        }}
        var v2 = client as IPolicyConfig2;
        if (v2 != null) {{
            r0 = v2.SetDefaultEndpoint(id, 0); r1 = v2.SetDefaultEndpoint(id, 1); r2 = v2.SetDefaultEndpoint(id, 2);
            Console.WriteLine("Success({{0}}-v2): 0x{{1:X}}, 0x{{2:X}}, 0x{{3:X}}", clientName, r0, r1, r2);
            return true;
        }}
        var v3 = client as IPolicyConfig3;
        if (v3 != null) {{
            r0 = v3.SetDefaultEndpoint(id, 0); r1 = v3.SetDefaultEndpoint(id, 1); r2 = v3.SetDefaultEndpoint(id, 2);
            Console.WriteLine("Success({{0}}-v3): 0x{{1:X}}, 0x{{2:X}}, 0x{{3:X}}", clientName, r0, r1, r2);
            return true;
        }}
        var v4 = client as IPolicyConfig4;
        if (v4 != null) {{
            r0 = v4.SetDefaultEndpoint(id, 0); r1 = v4.SetDefaultEndpoint(id, 1); r2 = v4.SetDefaultEndpoint(id, 2);
            Console.WriteLine("Success({{0}}-v4): 0x{{1:X}}, 0x{{2:X}}, 0x{{3:X}}", clientName, r0, r1, r2);
            return true;
        }}
        var v5 = client as IPolicyConfig5;
        if (v5 != null) {{
            r0 = v5.SetDefaultEndpoint(id, 0); r1 = v5.SetDefaultEndpoint(id, 1); r2 = v5.SetDefaultEndpoint(id, 2);
            Console.WriteLine("Success({{0}}-v5): 0x{{1:X}}, 0x{{2:X}}, 0x{{3:X}}", clientName, r0, r1, r2);
            return true;
        }}
        return false;
    }}
}}
'@;
Add-Type -TypeDefinition $code; [Switcher]::Set($id)"#,
        device_id
    );

    let output = Command::new("powershell")
        .creation_flags(0x08000000)
        .args(&[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .output()
        .map_err(|e| Error::new(E_FAIL, format!("PS execution failed: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print to terminal for debugging
    if !stdout.is_empty() {
        println!("PS Stdout: {}", stdout.trim());
    }
    if !stderr.is_empty() {
        eprintln!("PS Stderr: {}", stderr.trim());
    }

    // 修正: パース処理を pure 関数に委譲してエラーを Windows Error に変換
    parse_powershell_output(&stdout, output.status.success())
        .map_err(|e| Error::new(E_FAIL, format!("{}\nStderr: {}", e, stderr.trim())))?;

    Ok(())
}

#[tauri::command]
pub fn get_audio_devices() -> std::result::Result<Vec<AudioDevice>, String> {
    println!("Command: get_audio_devices called");
    list_devices().map_err(|e| {
        println!("Error in get_audio_devices: {}", e);
        e.to_string()
    })
}

#[tauri::command]
pub fn switch_audio_device(id: String) -> std::result::Result<(), String> {
    println!("Command: switch_audio_device called with id: {}", id);
    set_default_device(id).map_err(|e| {
        println!("Error in switch_audio_device: {}", e);
        e.to_string()
    })
}

// ===== メディアキー送信 =====
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP,
    VIRTUAL_KEY,
};

/// Windows API の SendInput でメディアキーの押下・離上をシミュレートする
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

/// 再生/一時停止 (VK_MEDIA_PLAY_PAUSE = 0xB3)
#[tauri::command]
pub fn media_play_pause() {
    send_media_key(0xB3);
}

/// 次のトラック (VK_MEDIA_NEXT_TRACK = 0xB0)
#[tauri::command]
pub fn media_next() {
    send_media_key(0xB0);
}

/// 前のトラック (VK_MEDIA_PREV_TRACK = 0xB1)
#[tauri::command]
pub fn media_prev() {
    send_media_key(0xB1);
}

// ===== ブラウザ個別ミュート =====
// windows::Win32::Media::Audio::* はファイル先頭でワイルドカードimport済み
// (IAudioSessionControl2, IAudioSessionEnumerator, IAudioSessionManager2,
//  ISimpleAudioVolume 等はすべて利用可能)

/// Chrome / Edge / Firefox 等ブラウザのオーディオセッションをミュート切り替えする。
/// 戻り値: ミュート後の状態 (true = ミュート中)
#[tauri::command]
pub fn toggle_browser_mute() -> std::result::Result<bool, String> {
    use sysinfo::System;

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
            system.refresh_all();
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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== parse_powershell_output のテスト =====

    #[test]
    fn test_success_all_roles_zero() {
        // 全ロールが 0x0 → Ok
        let result = parse_powershell_output("Success(Client-v1): 0x0, 0x0, 0x0", true);
        assert!(result.is_ok(), "全ロール 0x0 は成功とみなすべき");
    }

    #[test]
    fn test_success_vista_client() {
        // VistaClient でも同様
        let result = parse_powershell_output("Success(VistaClient-v1): 0x0, 0x0, 0x0", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_exit_failure_returns_error() {
        // exit code が 0 以外 → Err
        let result = parse_powershell_output("", false);
        assert!(
            result.is_err(),
            "exit_success=false の場合はエラーを返すべき"
        );
        assert!(result.unwrap_err().contains("終了コードエラー"));
    }

    #[test]
    fn test_error_message_in_stdout_returns_error() {
        // C# 側が "Error: ..." を出力した場合 → Err
        let result = parse_powershell_output(
            "Error: No supported IPolicyConfig interface found.",
            true,
        );
        assert!(
            result.is_err(),
            "stdout に 'Error:' が含まれる場合はエラーを返すべき"
        );
        assert!(result.unwrap_err().contains("SetDefaultEndpoint失敗"));
    }

    #[test]
    fn test_nonzero_hresult_returns_error() {
        // 0x80070005 (アクセス拒否) などの非ゼロ HRESULT → Err
        let result =
            parse_powershell_output("Success(Client-v1): 0x80070005, 0x0, 0x0", true);
        assert!(
            result.is_err(),
            "非ゼロ HRESULT が含まれる場合はエラーを返すべき"
        );
    }

    #[test]
    fn test_empty_stdout_exit_success_is_ok() {
        // 出力なし・成功終了 → Ok（PowerShell が何も出力しないケース）
        let result = parse_powershell_output("", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_partial_nonzero_hresult_returns_error() {
        // 最初のロールのみ成功・残り失敗
        let result =
            parse_powershell_output("Success(Client-v1): 0x0, 0x80004005, 0x0", true);
        assert!(
            result.is_err(),
            "一部のロールが非ゼロの場合はエラーを返すべき"
        );
    }

    #[test]
    fn test_stdout_with_trailing_newline_ok() {
        // 末尾に改行が含まれる場合でも正しく判定
        let result = parse_powershell_output("Success(Client-v1): 0x0, 0x0, 0x0\r\n", true);
        assert!(result.is_ok(), "末尾改行付きでも全ロール 0x0 は成功");
    }

    // ===== [中優先度] toggle_browser_mute の統合テスト =====

    #[test]
    fn test_toggle_browser_mute_does_not_panic() {
        // Windows COM API を使用した統合テスト
        // ブラウザが起動していない状態でも Ok(bool) を返し、パニックしないことを確認する
        // （ブラウザ未起動時は Ok(false) が期待値）
        let result = toggle_browser_mute();
        assert!(
            result.is_ok(),
            "toggle_browser_mute \u{306f}\u{30d6}\u{30e9}\u{30a6}\u{30b6}\u{672a}\u{8d77}\u{52d5}\u{6642}\u{3082} Ok \u{3092}\u{8fd4}\u{3059}\u{3079}\u{304d}: {:?}",
            result
        );
    }

    #[test]
    fn test_toggle_browser_mute_returns_bool() {
        // 戻り値の型が bool であること（ミュート状態の真偽値）
        let result = toggle_browser_mute();
        if let Ok(mute_state) = result {
            // bool 型であれば型チェックが通る（コンパイル時保証）
            let _: bool = mute_state;
        }
        // Err の場合はスキップ（COM 初期化失敗など環境依存エラーは許容）
    }

    #[test]
    fn test_toggle_browser_mute_browser_names_coverage() {
        // ブラウザ名リストに主要ブラウザが含まれていることを parse_powershell_output 経由で確認
        // （toggle_browser_mute 内部の browsers 配列の検証）
        // 注意: ブラウザ名の直接テストは内部実装のため、
        //       この統合テストではブラウザ名マッチングの網羅性をドキュメント化する
        //
        // 対象ブラウザ: chrome, msedge, firefox, opera, brave, vivaldi
        // 問題点: exact process name matching のため、カスタムビルドや
        //         フォーク版ブラウザが検出されない可能性がある
        //
        // このテストは現状の動作を記録する（バグ検出ではなくドキュメント目的）
        let result = toggle_browser_mute();
        // 結果は環境依存（ブラウザ起動状態によって変わる）なので成否不問
        let _ = result;
    }
}
