use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

/// カテゴリ未指定の古いエントリ（category追加前のJSON）は "game" 扱いにする
fn default_category() -> String {
    "game".to_string()
}

/// category 文字列を正規化する（"app" 以外はすべて "game" に丸める）
fn normalize_category(category: &str) -> String {
    if category == "app" {
        "app".to_string()
    } else {
        "game".to_string()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ManualGame {
    pub id: String,
    pub name: String,
    pub exe_path: String,
    /// "game" = 非Steamゲーム（Games>その他）, "app" = 一般アプリ（アプリタブ）
    #[serde(default = "default_category")]
    pub category: String,
}

/// 手動ゲーム一覧の保存先パスを返す（app_data_dir/manual_games.json）
fn get_manual_games_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path.join("manual_games.json"))
}

/// 実行ファイルパスを検証する（テスト可能な純粋関数）
/// 要件: 不正改造 - パストラバーサル・NULバイト・不正な拡張子を拒否する
pub fn validate_exe_path(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("実行ファイルパスが空です".to_string());
    }
    // パストラバーサル攻撃の検出
    if path.contains("..") {
        return Err(format!(
            "パスに '..' が含まれており、パストラバーサルの可能性があります: '{}'",
            path
        ));
    }
    // NULバイトインジェクションの検出
    if path.contains('\0') {
        return Err("パスにNULバイトが含まれています".to_string());
    }
    // 起動可能な拡張子のみ許可する
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    match ext.as_deref() {
        Some("exe") | Some("lnk") | Some("bat") | Some("cmd") | Some("url") => Ok(()),
        _ => Err(format!(
            "対応していない拡張子です（exe/lnk/bat/cmd/url のみ許可）: '{}'",
            path
        )),
    }
}

/// JSONテキストから手動ゲーム一覧をパースする（テスト可能な純粋関数）
fn parse_manual_games_content(content: &str) -> Result<Vec<ManualGame>, String> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(content)
        .map_err(|e| format!("手動ゲームのJSONパースに失敗しました: {}", e))
}

/// 一意なIDを生成する（タイムスタンプのナノ秒ベース）
fn generate_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("manual_{}", nanos)
}

fn read_manual_games(app_handle: &AppHandle) -> Result<Vec<ManualGame>, String> {
    let path = get_manual_games_path(app_handle)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_manual_games_content(&content)
}

fn write_manual_games(app_handle: &AppHandle, games: &[ManualGame]) -> Result<(), String> {
    let path = get_manual_games_path(app_handle)?;
    let content = serde_json::to_string(games).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_manual_games(app_handle: AppHandle) -> Result<Vec<ManualGame>, String> {
    read_manual_games(&app_handle)
}

#[tauri::command]
pub async fn add_manual_game(
    app_handle: AppHandle,
    name: String,
    exe_path: String,
    category: String,
) -> Result<ManualGame, String> {
    // 不正改造防止: 保存前にパスを検証する
    validate_exe_path(&exe_path)?;

    // 実在チェック
    if !std::path::Path::new(&exe_path).exists() {
        return Err(format!("ファイルが存在しません: '{}'", exe_path));
    }

    let trimmed_name = name.trim();
    let final_name = if trimmed_name.is_empty() {
        // 名前が空ならファイル名（拡張子なし）を使う
        std::path::Path::new(&exe_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    } else {
        trimmed_name.to_string()
    };

    let mut games = read_manual_games(&app_handle)?;
    let game = ManualGame {
        id: generate_id(),
        name: final_name,
        exe_path,
        category: normalize_category(&category),
    };
    games.push(game.clone());
    write_manual_games(&app_handle, &games)?;
    Ok(game)
}

#[tauri::command]
pub async fn remove_manual_game(app_handle: AppHandle, id: String) -> Result<(), String> {
    let mut games = read_manual_games(&app_handle)?;
    let before = games.len();
    games.retain(|g| g.id != id);
    if games.len() == before {
        return Err(format!("指定されたIDのゲームが見つかりません: '{}'", id));
    }
    write_manual_games(&app_handle, &games)
}

#[tauri::command]
pub async fn launch_manual_game(app_handle: AppHandle, id: String) -> Result<(), String> {
    let games = read_manual_games(&app_handle)?;
    let game = games
        .into_iter()
        .find(|g| g.id == id)
        .ok_or_else(|| format!("指定されたIDのゲームが見つかりません: '{}'", id))?;

    // 不正改造防止: 起動前に再検証する（保存後にJSONが書き換えられた場合の保険）
    validate_exe_path(&game.exe_path)?;

    if let Err(e) = opener::open(&game.exe_path) {
        eprintln!("Failed to launch manual game: {}", e);
        return Err(e.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== [セキュリティ] validate_exe_path のテスト =====

    #[test]
    fn test_valid_exe_path_accepted() {
        let valid = [
            "C:\\Games\\game.exe",
            "C:\\Users\\user\\Desktop\\shortcut.lnk",
            "D:\\tools\\run.bat",
            "C:\\a\\b.cmd",
            "C:\\links\\game.url",
        ];
        for p in &valid {
            assert!(
                validate_exe_path(p).is_ok(),
                "正規のパス '{}' は検証を通過すべき",
                p
            );
        }
    }

    #[test]
    fn test_empty_exe_path_rejected() {
        assert!(validate_exe_path("").is_err(), "空のパスはエラーを返すべき");
        assert!(validate_exe_path("   ").is_err(), "空白のみのパスはエラーを返すべき");
    }

    #[test]
    fn test_exe_path_with_traversal_rejected() {
        let malicious = "C:\\Games\\..\\..\\Windows\\System32\\cmd.exe";
        assert!(
            validate_exe_path(malicious).is_err(),
            "'..' を含むパスはパストラバーサル防止のためエラーを返すべき"
        );
    }

    #[test]
    fn test_exe_path_with_null_byte_rejected() {
        let malicious = "C:\\safe\\game.exe\0C:\\evil\\malware.exe";
        assert!(
            validate_exe_path(malicious).is_err(),
            "NULバイトを含むパスはエラーを返すべき"
        );
    }

    #[test]
    fn test_exe_path_disallowed_extension_rejected() {
        // 任意ファイル実行・スクリプト混入防止のため許可外拡張子は拒否
        let cases = [
            "C:\\malware\\evil.dll",
            "C:\\docs\\readme.txt",
            "C:\\scripts\\evil.ps1",
            "C:\\scripts\\evil.vbs",
            "C:\\noext\\game",
        ];
        for p in &cases {
            assert!(
                validate_exe_path(p).is_err(),
                "許可外拡張子 '{}' はエラーを返すべき",
                p
            );
        }
    }

    #[test]
    fn test_exe_path_extension_case_insensitive() {
        assert!(validate_exe_path("C:\\Games\\GAME.EXE").is_ok());
        assert!(validate_exe_path("C:\\Games\\Game.Lnk").is_ok());
    }

    // ===== parse_manual_games_content のテスト =====

    #[test]
    fn test_parse_empty_content_returns_empty() {
        assert!(parse_manual_games_content("").unwrap().is_empty());
        assert!(parse_manual_games_content("   ").unwrap().is_empty());
    }

    #[test]
    fn test_parse_valid_games_json() {
        let json = r#"[{"id":"manual_1","name":"My Game","exe_path":"C:\\Games\\g.exe","category":"game"}]"#;
        let result = parse_manual_games_content(json);
        assert!(result.is_ok());
        let games = result.unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "manual_1");
        assert_eq!(games[0].name, "My Game");
        assert_eq!(games[0].exe_path, "C:\\Games\\g.exe");
        assert_eq!(games[0].category, "game");
    }

    #[test]
    fn test_parse_legacy_json_without_category_defaults_to_game() {
        // category 追加前に保存された古いJSON（categoryなし）も読めること
        let json = r#"[{"id":"manual_1","name":"Old Entry","exe_path":"C:\\Games\\g.exe"}]"#;
        let games = parse_manual_games_content(json).unwrap();
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].category, "game", "categoryなしの古いエントリは game 既定にすべき");
    }

    #[test]
    fn test_parse_app_category_json() {
        let json = r#"[{"id":"manual_2","name":"Discord","exe_path":"C:\\Apps\\Discord.exe","category":"app"}]"#;
        let games = parse_manual_games_content(json).unwrap();
        assert_eq!(games[0].category, "app");
    }

    #[test]
    fn test_normalize_category() {
        assert_eq!(normalize_category("app"), "app");
        assert_eq!(normalize_category("game"), "game");
        // 未知の値・空文字は game に丸める
        assert_eq!(normalize_category(""), "game");
        assert_eq!(normalize_category("APP"), "game");
        assert_eq!(normalize_category("foo"), "game");
    }

    #[test]
    fn test_parse_empty_array_json() {
        let result = parse_manual_games_content("[]");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_invalid_json_returns_error() {
        let result = parse_manual_games_content("{ not valid json !!");
        assert!(result.is_err(), "壊れたJSONはエラーを返すべき");
        assert!(result.unwrap_err().contains("JSONパースに失敗"));
    }

    #[test]
    fn test_parse_wrong_type_returns_error() {
        // オブジェクト（配列ではない）はエラー
        let result = parse_manual_games_content(r#"{"id":"x"}"#);
        assert!(result.is_err(), "配列以外のJSONはエラーを返すべき");
    }

    #[test]
    fn test_parse_japanese_game_name() {
        let json = r#"[{"id":"manual_1","name":"モンスターハンター","exe_path":"C:\\Games\\mh.exe"}]"#;
        let games = parse_manual_games_content(json).unwrap();
        assert_eq!(games[0].name, "モンスターハンター");
    }

    // ===== generate_id のテスト =====

    #[test]
    fn test_generate_id_has_prefix() {
        let id = generate_id();
        assert!(id.starts_with("manual_"), "生成IDは manual_ 接頭辞を持つべき: {}", id);
    }
}
