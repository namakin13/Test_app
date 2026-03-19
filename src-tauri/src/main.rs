// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_manager;
mod steam_manager;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use audio_manager::{list_devices, set_default_device, AudioDevice};
use steam_manager::{get_steam_games, launch_steam_app, launch_steam_game, get_local_steam_image};

#[tauri::command]
fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    println!("Command: get_audio_devices called");
    list_devices().map_err(|e| {
        println!("Error in get_audio_devices: {}", e);
        e.to_string()
    })
}

#[tauri::command]
fn switch_audio_device(id: String) -> Result<(), String> {
    println!("Command: switch_audio_device called with id: {}", id);
    set_default_device(id).map_err(|e| {
        println!("Error in switch_audio_device: {}", e);
        e.to_string()
    })
}

fn get_icons_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path.join("custom_icons.json"))
}

#[tauri::command]
async fn get_custom_icons(app_handle: AppHandle) -> Result<HashMap<String, String>, String> {
    let path = get_icons_path(&app_handle)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let icons: HashMap<String, String> = serde_json::from_str(&content).unwrap_or_default();
    Ok(icons)
}

#[tauri::command]
async fn save_custom_icon(
    app_handle: AppHandle,
    appid: String,
    base64_data: String,
) -> Result<(), String> {
    let mut icons = get_custom_icons(app_handle.clone()).await?;
    icons.insert(appid, base64_data);
    let path = get_icons_path(&app_handle)?;
    let content = serde_json::to_string(&icons).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

use base64::{engine::general_purpose, Engine as _};

#[tauri::command]
async fn save_custom_icon_from_path(
    app_handle: AppHandle,
    appid: String,
    file_path: String,
) -> Result<(), String> {
    // Read the file from the given path
    let file_data = fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Convert to base64
    let base64_str = general_purpose::STANDARD.encode(&file_data);

    // Determine mime type from extension
    let path = std::path::Path::new(&file_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let mime_type = match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "jpeg",
        "gif" => "gif",
        "webp" => "webp",
        _ => "png", // default to png
    };

    let base64_data_uri = format!("data:image/{};base64,{}", mime_type, base64_str);

    // Save it using the existing save logic
    save_custom_icon(app_handle, appid, base64_data_uri).await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_audio_devices,
            switch_audio_device,
            get_steam_games,
            launch_steam_game,
            launch_steam_app,
            get_custom_icons,
            save_custom_icon,
            save_custom_icon_from_path,
            get_local_steam_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
