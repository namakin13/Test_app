use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::command;
use winreg::enums::*;
use winreg::RegKey;

#[derive(Debug, Serialize, Deserialize)]
pub struct SteamGame {
    pub name: String,
    pub appid: String,
    pub header_image_url: String,
}

/// Retrieve the Steam installation path from the Windows Registry
fn get_steam_install_path() -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let steam_key = hkcu.open_subkey("SOFTWARE\\Valve\\Steam").ok()?;
    let steam_path: String = steam_key.get_value("SteamPath").ok()?;
    Some(steam_path)
}

/// Parse a simple generic VDF (.acf) file to extract AppID and Name
fn parse_acf_file(path: &PathBuf) -> Option<SteamGame> {
    let contents = fs::read_to_string(path).ok()?;
    let mut name = String::new();
    let mut appid = String::new();

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.to_lowercase().starts_with("\"name\"") {
            if let Some(val) = extract_value(trimmed) {
                name = val;
            }
        } else if trimmed.to_lowercase().starts_with("\"appid\"") {
            if let Some(val) = extract_value(trimmed) {
                appid = val;
            }
        }

        if !name.is_empty() && !appid.is_empty() {
            // 正方形アイコン表示に最適な縦長画像をCDNフォールバック用として設定
            let header_image_url = format!(
                "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/{}/library_600x900.jpg",
                appid
            );
            return Some(SteamGame {
                name,
                appid,
                header_image_url,
            });
        }
    }
    None
}

/// Helper function to extract the value from a line like `"name" "GameTitle"`
fn extract_value(line: &str) -> Option<String> {
    let parts: Vec<&str> = line
        .split('"')
        .filter(|s| !s.is_empty() && s.trim() != "")
        .collect();
    if parts.len() >= 2 {
        Some(parts.last()?.to_string())
    } else {
        None
    }
}

#[command]
pub async fn get_steam_games() -> Result<Vec<SteamGame>, String> {
    println!("Command: get_steam_games called");
    let mut games = Vec::new();

    let steam_path = match get_steam_install_path() {
        Some(path) => path,
        None => return Ok(games), // Steam not found, return empty
    };

    let steamapps_path = PathBuf::from(steam_path).join("steamapps");

    if let Ok(entries) = fs::read_dir(steamapps_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("acf") {
                if let Some(game) = parse_acf_file(&path) {
                    // Filter out non-games or common steam redistributables if needed
                    games.push(game);
                }
            }
        }
    }

    Ok(games)
}

#[command]
pub async fn launch_steam_game(appid: String) -> Result<(), String> {
    println!("Command: launch_steam_game called with id: {}", appid);
    let url = format!("steam://run/{}", appid);

    // Use opener or direct command to launch URI scheme
    if let Err(e) = opener::open(&url) {
        eprintln!("Failed to launch game: {}", e);
        return Err(e.to_string());
    }

    Ok(())
}

#[command]
pub async fn launch_steam_app() -> Result<(), String> {
    println!("Command: launch_steam_app called");
    if let Err(e) = opener::open("steam://open/main") {
        eprintln!("Failed to launch Steam: {}", e);
        return Err(e.to_string());
    }
    Ok(())
}

use base64::{engine::general_purpose, Engine as _};

fn find_icon_from_shortcuts(appid: &str) -> Option<String> {
    let mut search_dirs = Vec::new();

    if let Ok(public_dir) = std::env::var("PUBLIC") {
        search_dirs.push(PathBuf::from(&public_dir).join("Desktop"));
        search_dirs.push(PathBuf::from(&public_dir).join("デスクトップ"));
    }

    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        search_dirs.push(PathBuf::from(&user_profile).join("Desktop"));
        search_dirs.push(PathBuf::from(&user_profile).join("デスクトップ"));
        search_dirs.push(PathBuf::from(&user_profile).join("OneDrive").join("Desktop"));
        search_dirs.push(PathBuf::from(&user_profile).join("OneDrive").join("デスクトップ"));
        search_dirs.push(
            PathBuf::from(&user_profile)
                .join("AppData")
                .join("Roaming")
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Steam"),
        );
    }

    if let Ok(program_data) = std::env::var("ProgramData") {
        search_dirs.push(
            PathBuf::from(&program_data)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Steam"),
        );
    }

    let search_url = format!("URL=steam://rungameid/{}", appid);

    for dir in search_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("url") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if content.contains(&search_url) {
                            for line in content.lines() {
                                if line.starts_with("IconFile=") {
                                    let icon_path = line.trim_start_matches("IconFile=").trim();
                                    return Some(icon_path.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

#[command]
pub async fn get_local_steam_image(appid: String) -> Result<Option<String>, String> {
    let steam_path = match get_steam_install_path() {
        Some(path) => path,
        None => return Ok(None),
    };

    let librarycache_dir = PathBuf::from(&steam_path)
        .join("appcache")
        .join("librarycache");

    // icon.jpg を最優先に、サブディレクトリ構造（新しいSteam）をチェック
    let subdir_candidates = [
        (librarycache_dir.join(&appid).join("icon.jpg"), "png"),
        (librarycache_dir.join(&appid).join("header.jpg"), "jpeg"),
        (librarycache_dir.join(&appid).join("library_600x900.jpg"), "jpeg"),
        (librarycache_dir.join(&appid).join("library_header.jpg"), "jpeg"),
        (librarycache_dir.join(&appid).join("logo.png"), "png"),
    ];

    // フラット構造（古いSteam: {appid}_icon.jpg 形式）もチェック
    let flat_candidates = [
        (librarycache_dir.join(format!("{}_icon.jpg", appid)), "png"),
        (librarycache_dir.join(format!("{}_header.jpg", appid)), "jpeg"),
        (librarycache_dir.join(format!("{}_library_600x900.jpg", appid)), "jpeg"),
        (librarycache_dir.join(format!("{}_library_header.jpg", appid)), "jpeg"),
    ];

    // Priority 1: サブディレクトリ構造 → フラット構造の順でチェック
    for (file_path, mime_type) in subdir_candidates.iter().chain(flat_candidates.iter()) {
        if file_path.exists() {
            if let Ok(file_data) = fs::read(file_path) {
                if file_data.len() > 16 {
                    let base64_str = general_purpose::STANDARD.encode(&file_data);
                    return Ok(Some(format!("data:image/{};base64,{}", mime_type, base64_str)));
                }
            }
        }
    }

    // Priority 2: Check desktop/start menu shortcuts for IconFile path
    if let Some(icon_path) = find_icon_from_shortcuts(&appid) {
        let path = PathBuf::from(&icon_path);
        if path.exists() {
            if let Ok(file_data) = fs::read(&path) {
                let base64_str = general_purpose::STANDARD.encode(&file_data);
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("ico").to_lowercase();
                
                let mime_type = match ext.as_str() {
                    "png" => "png",
                    "jpg" | "jpeg" => "jpeg",
                    "ico" => "x-icon",
                    _ => "x-icon",
                };
                return Ok(Some(format!("data:image/{};base64,{}", mime_type, base64_str)));
            }
        }
    }

    Ok(None)
}

