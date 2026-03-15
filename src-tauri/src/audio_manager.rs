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
use windows::Win32::UI::Shell::PropertiesSystem::*;

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

    if !output.status.success() {
        return Err(Error::new(
            E_FAIL,
            format!(
                "PS status error: {}\nStderr: {}\nStdout: {}",
                output.status, stderr, stdout
            ),
        ));
    }

    if stdout.contains("Error:") {
        return Err(Error::new(
            E_FAIL,
            format!("SetDefaultEndpoint failed: {}", stdout.trim()),
        ));
    }

    // Check if any role failed (returned anything other than 0x0)
    if stdout.contains("0x") {
        let parts: Vec<&str> = stdout.split(": ").collect();
        if parts.len() > 1 {
            let codes: Vec<&str> = parts[1].split(", ").collect();
            for code in codes {
                if !code.contains("0x0") && code.contains("0x") {
                    return Err(Error::new(
                        E_FAIL,
                        format!("One or more roles failed to switch: {}", stdout.trim()),
                    ));
                }
            }
        }
    }

    Ok(())
}
