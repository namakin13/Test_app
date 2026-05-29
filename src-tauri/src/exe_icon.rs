use base64::{engine::general_purpose, Engine as _};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Controls::{IImageList, ILD_TRANSPARENT};
use windows::Win32::UI::Shell::{
    SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SYSICONINDEX,
    SHIL_EXTRALARGE,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

use crate::manual_game_manager::validate_exe_path;

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// パスから 48x48 (EXTRALARGE) アイコンの HICON を取得する。
/// 失敗時は大アイコン (32x32) にフォールバックする。
unsafe fn get_hicon(path: &str) -> Option<HICON> {
    let wide = to_wide(path);

    // まず EXTRALARGE (48x48) をシステムイメージリストから取得する
    let mut shfi = SHFILEINFOW::default();
    let res = SHGetFileInfoW(
        PCWSTR(wide.as_ptr()),
        FILE_FLAGS_AND_ATTRIBUTES(0),
        Some(&mut shfi),
        std::mem::size_of::<SHFILEINFOW>() as u32,
        SHGFI_SYSICONINDEX,
    );
    if res != 0 {
        if let Ok(iml) = SHGetImageList::<IImageList>(SHIL_EXTRALARGE as i32) {
            if let Ok(hicon) = iml.GetIcon(shfi.iIcon, ILD_TRANSPARENT.0 as u32) {
                if !hicon.is_invalid() {
                    return Some(hicon);
                }
            }
        }
    }

    // フォールバック: 大アイコン (32x32)
    let mut shfi2 = SHFILEINFOW::default();
    let res2 = SHGetFileInfoW(
        PCWSTR(wide.as_ptr()),
        FILE_FLAGS_AND_ATTRIBUTES(0),
        Some(&mut shfi2),
        std::mem::size_of::<SHFILEINFOW>() as u32,
        SHGFI_ICON | SHGFI_LARGEICON,
    );
    if res2 != 0 && !shfi2.hIcon.is_invalid() {
        return Some(shfi2.hIcon);
    }

    None
}

/// HICON を RGBA ピクセルに変換し PNG バイト列にエンコードする
unsafe fn hicon_to_png(hicon: HICON) -> Option<Vec<u8>> {
    let mut info = ICONINFO::default();
    if GetIconInfo(hicon, &mut info).is_err() {
        return None;
    }
    let hbm_color = info.hbmColor;
    let hbm_mask = info.hbmMask;

    let cleanup_bitmaps = || {
        if !hbm_color.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(hbm_color.0));
        }
        if !hbm_mask.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(hbm_mask.0));
        }
    };

    if hbm_color.is_invalid() {
        cleanup_bitmaps();
        return None;
    }

    // ビットマップの寸法を取得
    let mut bmp = BITMAP::default();
    let got = GetObjectW(
        HGDIOBJ(hbm_color.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bmp as *mut _ as *mut _),
    );
    if got == 0 || bmp.bmWidth <= 0 || bmp.bmHeight <= 0 {
        cleanup_bitmaps();
        return None;
    }
    let width = bmp.bmWidth;
    let height = bmp.bmHeight;

    // 32bpp トップダウン DIB として読み出す
    let mut bmi = BITMAPINFO::default();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width;
    bmi.bmiHeader.biHeight = -height; // 負値 = トップダウン
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB.0 as u32;

    let pixel_count = (width * height) as usize;
    let mut buf = vec![0u8; pixel_count * 4];

    let hdc = GetDC(None);
    let scanned = GetDIBits(
        hdc,
        hbm_color,
        0,
        height as u32,
        Some(buf.as_mut_ptr() as *mut _),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    ReleaseDC(None, hdc);
    cleanup_bitmaps();

    if scanned == 0 {
        return None;
    }

    // アルファが全て 0 の場合（アルファ非対応アイコン）は不透明扱いにする
    let any_alpha = buf.chunks_exact(4).any(|px| px[3] != 0);

    // BGRA -> RGBA 変換
    let mut rgba = vec![0u8; buf.len()];
    for (i, px) in buf.chunks_exact(4).enumerate() {
        rgba[i * 4] = px[2]; // R
        rgba[i * 4 + 1] = px[1]; // G
        rgba[i * 4 + 2] = px[0]; // B
        rgba[i * 4 + 3] = if any_alpha { px[3] } else { 255 };
    }

    // PNG エンコード
    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, width as u32, height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&rgba).ok()?;
    }

    Some(png_data)
}

/// 実行ファイル/ショートカットからアイコンを抽出し、PNG の data URI を返す
#[tauri::command]
pub async fn get_exe_icon(exe_path: String) -> Result<Option<String>, String> {
    // 不正改造防止: アイコン抽出前にパスを検証する
    validate_exe_path(&exe_path)?;

    if !std::path::Path::new(&exe_path).exists() {
        return Ok(None);
    }

    let png_bytes = unsafe {
        match get_hicon(&exe_path) {
            Some(hicon) => {
                let png = hicon_to_png(hicon);
                let _ = DestroyIcon(hicon);
                png
            }
            None => None,
        }
    };

    match png_bytes {
        Some(bytes) if bytes.len() > 16 => {
            let base64_str = general_purpose::STANDARD.encode(&bytes);
            Ok(Some(format!("data:image/png;base64,{}", base64_str)))
        }
        _ => Ok(None),
    }
}
