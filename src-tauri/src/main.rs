// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_manager;
mod icon_manager;
mod steam_manager;

use audio_manager::{
    get_audio_devices, get_device_volume, media_next, media_play_pause, media_prev,
    set_device_volume, switch_audio_device, toggle_browser_mute,
};
use icon_manager::{get_custom_icons, save_custom_icon, save_custom_icon_from_path};
use steam_manager::{get_local_steam_image, get_steam_games, launch_steam_app, launch_steam_game};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_audio_devices,
            switch_audio_device,
            get_device_volume,
            set_device_volume,
            media_play_pause,
            media_next,
            media_prev,
            toggle_browser_mute,
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
