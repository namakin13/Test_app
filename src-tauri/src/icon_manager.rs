use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn get_icons_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path.join("custom_icons.json"))
}

#[tauri::command]
pub async fn get_custom_icons(app_handle: AppHandle) -> Result<HashMap<String, String>, String> {
    let path = get_icons_path(&app_handle)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let icons: HashMap<String, String> = serde_json::from_str(&content).unwrap_or_default();
    Ok(icons)
}

#[tauri::command]
pub async fn save_custom_icon(
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

#[tauri::command]
pub async fn save_custom_icon_from_path(
    app_handle: AppHandle,
    appid: String,
    file_path: String,
) -> Result<(), String> {
    let file_data = fs::read(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let base64_str = general_purpose::STANDARD.encode(&file_data);

    let path = std::path::Path::new(&file_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let mime_type = match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "jpeg",
        "gif" => "gif",
        "webp" => "webp",
        _ => "png",
    };

    let base64_data_uri = format!("data:image/{};base64,{}", mime_type, base64_str);
    save_custom_icon(app_handle, appid, base64_data_uri).await
}
