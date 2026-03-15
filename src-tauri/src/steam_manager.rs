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
            let header_image_url = format!(
                "https://cdn.akamai.steamstatic.com/steam/apps/{}/header.jpg",
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
