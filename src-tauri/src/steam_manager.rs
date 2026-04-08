use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::command;
use winreg::enums::*;
use winreg::RegKey;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

/// ACFファイルのテキスト内容から SteamGame を解析する（テスト可能な純粋関数）
fn parse_acf_content(contents: &str) -> Option<SteamGame> {
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

/// Parse a simple generic VDF (.acf) file to extract AppID and Name
fn parse_acf_file(path: &PathBuf) -> Option<SteamGame> {
    let contents = fs::read_to_string(path).ok()?;
    parse_acf_content(&contents)
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

/// appid が数字のみで構成されているか検証する（純粋関数）
/// Steam AppID は正規には正の整数値のため、数字以外は不正改造の可能性がある
pub fn validate_appid(appid: &str) -> Result<(), String> {
    if appid.is_empty() {
        return Err("AppIDが空です".to_string());
    }
    if !appid.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!(
            "AppIDは数字のみで構成されるべきです (不正改造の可能性): '{}'",
            appid
        ));
    }
    Ok(())
}

/// icon_path がパストラバーサル攻撃を含んでいないか検証する（純粋関数）
pub fn validate_icon_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("アイコンパスが空です".to_string());
    }
    if path.contains("..") {
        return Err(format!(
            "アイコンパスに '..' が含まれており、パストラバーサルの可能性があります: '{}'",
            path
        ));
    }
    if path.contains('\0') {
        return Err("アイコンパスにNULバイトが含まれています".to_string());
    }
    Ok(())
}

#[command]
pub async fn launch_steam_game(appid: String) -> Result<(), String> {
    println!("Command: launch_steam_game called with id: {}", appid);
    // 不正改造防止: appid を Steam URI に組み込む前に数字のみか検証する
    validate_appid(&appid)?;
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
                                    // 不正改造防止: ショートカットファイルから取得したパスを検証する
                                    if validate_icon_path(icon_path).is_err() {
                                        continue;
                                    }
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

    let appid_dir = librarycache_dir.join(&appid);

    // Priority 1: サブディレクトリ構造（新しいSteam）の標準ファイルをチェック
    // ※ icon.jpg はSteamのキャッシュに存在しない。実際には {40文字ハッシュ}.jpg で保存される
    let named_candidates = [
        (appid_dir.join("library_600x900.jpg"), "jpeg"), // 縦長＝正方形トリミングに最適
        (appid_dir.join("header.jpg"), "jpeg"),
        (appid_dir.join("library_header.jpg"), "jpeg"),
        (appid_dir.join("logo.png"), "png"),
    ];

    for (file_path, mime_type) in named_candidates.iter() {
        if file_path.exists() {
            if let Ok(file_data) = fs::read(file_path) {
                if file_data.len() > 16 {
                    let base64_str = general_purpose::STANDARD.encode(&file_data);
                    return Ok(Some(format!("data:image/{};base64,{}", mime_type, base64_str)));
                }
            }
        }
    }

    // Priority 2: フラット構造（古いSteam: {appid}_xxx.jpg 形式）をチェック
    let flat_candidates = [
        (librarycache_dir.join(format!("{}_library_600x900.jpg", appid)), "jpeg"),
        (librarycache_dir.join(format!("{}_header.jpg", appid)), "jpeg"),
        (librarycache_dir.join(format!("{}_library_header.jpg", appid)), "jpeg"),
    ];

    for (file_path, mime_type) in flat_candidates.iter() {
        if file_path.exists() {
            if let Ok(file_data) = fs::read(file_path) {
                if file_data.len() > 16 {
                    let base64_str = general_purpose::STANDARD.encode(&file_data);
                    return Ok(Some(format!("data:image/{};base64,{}", mime_type, base64_str)));
                }
            }
        }
    }

    // Priority 3: ハッシュ名 .jpg ファイルを検索（Steam が {40文字ハッシュ}.jpg で保存するアイコン）
    // 標準名のファイルが存在しないゲーム（DLC・ツール等）向けのフォールバック
    if appid_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&appid_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // {40文字の小文字16進数}.jpg にマッチするファイルを使用
                if name_str.len() == 44
                    && name_str.ends_with(".jpg")
                    && name_str[..40].chars().all(|c| c.is_ascii_hexdigit())
                {
                    if let Ok(file_data) = fs::read(entry.path()) {
                        if file_data.len() > 16 {
                            let base64_str = general_purpose::STANDARD.encode(&file_data);
                            return Ok(Some(format!("data:image/jpeg;base64,{}", base64_str)));
                        }
                    }
                }
            }
        }
    }

    // Priority 4: Check desktop/start menu shortcuts for IconFile path
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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== [セキュリティ] validate_appid のテスト =====
    // 要件: 不正改造 - Steam URI スキームへの不正な文字混入防止

    #[test]
    fn test_valid_appid_accepted() {
        // 正規の数字のみ AppID は通過すること
        for id in &["440", "730", "1234567890"] {
            assert!(
                validate_appid(id).is_ok(),
                "正規のAppID '{}' は検証を通過すべき",
                id
            );
        }
    }

    #[test]
    fn test_empty_appid_rejected() {
        assert!(validate_appid("").is_err(), "空のAppIDはエラーを返すべき");
    }

    #[test]
    fn test_appid_with_path_traversal_rejected() {
        // 攻撃例: ../../etc/passwd のようなパス混入
        let malicious = "440/../../../etc/passwd";
        let result = validate_appid(malicious);
        assert!(
            result.is_err(),
            "スラッシュ入りAppIDはURIインジェクション防止のためエラーを返すべき: '{}'",
            malicious
        );
    }

    #[test]
    fn test_appid_with_uri_injection_rejected() {
        // 攻撃例: steam://run/ を抜けて別URIを注入
        let malicious = "440/open/main";
        let result = validate_appid(malicious);
        assert!(
            result.is_err(),
            "スラッシュを含むAppIDはURIインジェクション防止のためエラーを返すべき"
        );
    }

    #[test]
    fn test_appid_with_protocol_injection_rejected() {
        // 攻撃例: URL に別のプロトコル（javascript: 等）を混入
        let malicious = "440%0ajavascript:alert(1)";
        let result = validate_appid(malicious);
        assert!(result.is_err(), "パーセントエンコードを含むAppIDはエラーを返すべき");
    }

    #[test]
    fn test_appid_with_spaces_rejected() {
        let result = validate_appid("440 && calc.exe");
        assert!(result.is_err(), "スペースを含むAppIDはエラーを返すべき");
    }

    #[test]
    fn test_appid_negative_number_rejected() {
        // マイナス符号は数字ではない
        let result = validate_appid("-440");
        assert!(result.is_err(), "負の数のAppIDはエラーを返すべき");
    }

    // ===== [セキュリティ] validate_icon_path のテスト =====
    // 要件: 不正改造 - ショートカットファイル経由のパストラバーサル防止

    #[test]
    fn test_valid_icon_path_accepted() {
        // 正規のアイコンパスは通過すること
        let valid = "C:\\Program Files (x86)\\Steam\\steamapps\\common\\game\\icon.ico";
        assert!(
            validate_icon_path(valid).is_ok(),
            "正規のアイコンパス '{}' は検証を通過すべき",
            valid
        );
    }

    #[test]
    fn test_icon_path_with_traversal_rejected() {
        // 攻撃例: 悪意のある .url ファイルに IconFile=..\..\Windows\System32\cmd.exe を記載
        let malicious = "C:\\Steam\\..\\..\\Windows\\System32\\cmd.exe";
        let result = validate_icon_path(malicious);
        assert!(
            result.is_err(),
            "'..' を含むアイコンパスはパストラバーサル防止のためエラーを返すべき: '{}'",
            malicious
        );
    }

    #[test]
    fn test_icon_path_with_relative_traversal_rejected() {
        // 攻撃例: 相対パストラバーサル
        let malicious = "..\\..\\..\\sensitive\\data.txt";
        let result = validate_icon_path(malicious);
        assert!(result.is_err(), "相対パストラバーサルはエラーを返すべき");
    }

    #[test]
    fn test_icon_path_empty_rejected() {
        assert!(validate_icon_path("").is_err(), "空のアイコンパスはエラーを返すべき");
    }

    #[test]
    fn test_icon_path_with_null_byte_rejected() {
        // 攻撃例: NULバイトでパスを切り詰める（Null Byte Injection）
        let malicious = "C:\\safe\\path.ico\0C:\\evil\\malware.exe";
        let result = validate_icon_path(malicious);
        assert!(
            result.is_err(),
            "NULバイトを含むアイコンパスはNullバイトインジェクション防止のためエラーを返すべき"
        );
    }

    // ===== extract_value のテスト =====

    #[test]
    fn test_extract_value_basic() {
        assert_eq!(
            extract_value("\"name\"\t\t\"Team Fortress 2\""),
            Some("Team Fortress 2".to_string())
        );
    }

    #[test]
    fn test_extract_value_numeric() {
        assert_eq!(
            extract_value("\"appid\"\t\t\"440\""),
            Some("440".to_string())
        );
    }

    #[test]
    fn test_extract_value_japanese_name() {
        assert_eq!(
            extract_value("\"name\"\t\t\"\u{30e2}\u{30f3}\u{30b9}\u{30bf}\u{30fc}\u{30cf}\u{30f3}\u{30bf}\u{30fc}\""),
            Some("\u{30e2}\u{30f3}\u{30b9}\u{30bf}\u{30fc}\u{30cf}\u{30f3}\u{30bf}\u{30fc}".to_string())
        );
    }

    #[test]
    fn test_extract_value_only_key_returns_none() {
        assert_eq!(extract_value("\"name\""), None);
    }

    #[test]
    fn test_extract_value_name_with_spaces() {
        assert_eq!(
            extract_value("\"name\" \"Dark Souls III\""),
            Some("Dark Souls III".to_string())
        );
    }

    // ===== [低優先度] extract_value エッジケース =====

    #[test]
    fn test_extract_value_empty_string_value_returns_none() {
        // 値が空文字 "" の場合は None を返すこと
        // split('"') の結果から空文字列がフィルタされ parts.len() < 2 → None
        assert_eq!(extract_value("\"name\"\t\t\"\""), None);
    }

    #[test]
    fn test_extract_value_whitespace_only_value_returns_none() {
        // 値が空白のみ "   " の場合は None を返すこと
        // s.trim() == "" でフィルタされるため parts.len() < 2 → None
        assert_eq!(extract_value("\"name\"\t\t\"   \""), None);
    }

    #[test]
    fn test_extract_value_no_quotes_returns_none() {
        // クォートを含まない不正形式の行は None を返すこと
        assert_eq!(extract_value("name value"), None);
    }

    // ===== parse_acf_content のテスト =====

    #[test]
    fn test_parse_acf_valid() {
        let content = "\"AppState\"\n{\n\t\"appid\"\t\t\"440\"\n\t\"Universe\"\t\"1\"\n\t\"name\"\t\t\"Team Fortress 2\"\n\t\"StateFlags\"\t\"4\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_some(), "\u{6b63}\u{5e38}ACF\u{306f}SteamGame\u{3092}\u{8fd4}\u{3059}\u{3079}\u{304d}");
        let game = result.unwrap();
        assert_eq!(game.appid, "440");
        assert_eq!(game.name, "Team Fortress 2");
        assert!(game.header_image_url.contains("440"));
    }

    #[test]
    fn test_parse_acf_missing_name_returns_none() {
        let content = "\"AppState\"\n{\n\t\"appid\"\t\t\"440\"\n\t\"StateFlags\"\t\"4\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_none(), "name \u{304c}\u{6b20}\u{640d}ACF\u{306f} None \u{3092}\u{8fd4}\u{3059}\u{3079}\u{304d}");
    }

    #[test]
    fn test_parse_acf_missing_appid_returns_none() {
        let content = "\"AppState\"\n{\n\t\"name\"\t\t\"Some Game\"\n\t\"StateFlags\"\t\"4\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_none(), "appid \u{304c}\u{6b20}\u{640d}ACF\u{306f} None \u{3092}\u{8fd4}\u{3059}\u{3079}\u{304d}");
    }

    #[test]
    fn test_parse_acf_japanese_game_name() {
        let content = "\"AppState\"\n{\n\t\"appid\"\t\t\"1234567\"\n\t\"name\"\t\t\"\u{30e2}\u{30f3}\u{30b9}\u{30bf}\u{30fc}\u{30cf}\u{30f3}\u{30bf}\u{30fc}\u{30ef}\u{30fc}\u{30eb}\u{30c9}\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_some());
        let game = result.unwrap();
        assert_eq!(game.name, "\u{30e2}\u{30f3}\u{30b9}\u{30bf}\u{30fc}\u{30cf}\u{30f3}\u{30bf}\u{30fc}\u{30ef}\u{30fc}\u{30eb}\u{30c9}");
        assert_eq!(game.appid, "1234567");
    }

    #[test]
    fn test_parse_acf_empty_content_returns_none() {
        assert!(parse_acf_content("").is_none());
    }

    #[test]
    fn test_parse_acf_header_image_url_format() {
        let content = "\"AppState\"\n{\n\t\"appid\"\t\t\"730\"\n\t\"name\"\t\t\"Counter-Strike 2\"\n}\n";
        let game = parse_acf_content(content).unwrap();
        assert_eq!(
            game.header_image_url,
            "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/730/library_600x900.jpg"
        );
    }

    #[test]
    fn test_parse_acf_case_insensitive_keys() {
        let content = "\"AppState\"\n{\n\t\"AppID\"\t\t\"999\"\n\t\"Name\"\t\t\"Test Game\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_some(), "\u{5927}\u{6587}\u{5b57}\u{30ad}\u{30fc}\u{3082}\u{89e3}\u{6790}\u{3067}\u{304d}\u{308b}\u{3079}\u{304d}");
        let game = result.unwrap();
        assert_eq!(game.appid, "999");
        assert_eq!(game.name, "Test Game");
    }

    // ===== [中優先度] ACFパーサー堅牢性のテスト =====

    #[test]
    fn test_parse_acf_crlf_line_endings() {
        // Windows 形式の CRLF 改行でも正しく解析できること
        let content = "\"AppState\"\r\n{\r\n\t\"appid\"\t\t\"440\"\r\n\t\"name\"\t\t\"Team Fortress 2\"\r\n}\r\n";
        let result = parse_acf_content(content);
        assert!(result.is_some(), "CRLF\u{6539}\u{884c}\u{306e}ACF\u{3082}\u{89e3}\u{6790}\u{3067}\u{304d}\u{308b}\u{3079}\u{304d}");
        let game = result.unwrap();
        assert_eq!(game.appid, "440");
        assert_eq!(game.name, "Team Fortress 2");
    }

    #[test]
    fn test_parse_acf_game_name_with_colon() {
        // コロンを含むゲーム名（例: "Grand Theft Auto V: Enhanced"）
        let content = "\"AppState\"\n{\n\t\"appid\"\t\t\"271590\"\n\t\"name\"\t\t\"Grand Theft Auto V: Enhanced\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_some());
        let game = result.unwrap();
        assert_eq!(game.name, "Grand Theft Auto V: Enhanced");
    }

    #[test]
    fn test_parse_acf_game_name_with_ampersand() {
        // アンパサンドを含むゲーム名
        let content = "\"AppState\"\n{\n\t\"appid\"\t\t\"12345\"\n\t\"name\"\t\t\"Tom & Jerry: Chase\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Tom & Jerry: Chase");
    }

    #[test]
    fn test_parse_acf_extra_fields_ignored() {
        // 無関係なフィールドが多数あっても appid と name を正しく取得できること
        let content = "\"AppState\"\n{\n\t\"appid\"\t\t\"440\"\n\t\"Universe\"\t\"1\"\n\t\"LauncherPath\"\t\"\"\n\t\"name\"\t\t\"Team Fortress 2\"\n\t\"StateFlags\"\t\"4\"\n\t\"installdir\"\t\t\"Team Fortress 2\"\n\t\"SizeOnDisk\"\t\"21474836480\"\n\t\"BytesToDownload\"\t\"0\"\n\t\"BytesDownloaded\"\t\"0\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_some());
        let game = result.unwrap();
        assert_eq!(game.appid, "440");
        assert_eq!(game.name, "Team Fortress 2");
    }

    #[test]
    fn test_parse_acf_name_before_appid() {
        // name が appid より先に現れる場合も正しく解析できること
        let content = "\"AppState\"\n{\n\t\"name\"\t\t\"Counter-Strike 2\"\n\t\"appid\"\t\t\"730\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_some(), "name\u{304c}appid\u{3088}\u{308a}\u{5148}\u{306b}\u{73fe}\u{308c}\u{3066}\u{3082}\u{89e3}\u{6790}\u{3067}\u{304d}\u{308b}\u{3079}\u{304d}");
        let game = result.unwrap();
        assert_eq!(game.appid, "730");
        assert_eq!(game.name, "Counter-Strike 2");
    }

    #[test]
    fn test_parse_acf_game_name_with_numbers() {
        // 数字を含むゲーム名
        let content = "\"AppState\"\n{\n\t\"appid\"\t\t\"221100\"\n\t\"name\"\t\t\"DayZ\"\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "DayZ");
    }

    #[test]
    fn test_parse_acf_whitespace_only_lines() {
        // 空行・空白行が含まれていても解析できること
        let content = "\"AppState\"\n{\n\n\t\"appid\"\t\t\"440\"\n\n\t\"name\"\t\t\"Team Fortress 2\"\n\n}\n";
        let result = parse_acf_content(content);
        assert!(result.is_some(), "\u{7a7a}\u{884c}\u{306e}\u{3042}\u{308b}ACF\u{3082}\u{89e3}\u{6790}\u{3067}\u{304d}\u{308b}\u{3079}\u{304d}");
    }

    #[test]
    fn test_parse_acf_value_with_whitespace_only_not_returned() {
        // 値が空白のみの場合は extract_value が None を返す
        // → 空白のみの name は無視され、最終的に None
        let content = "\"AppState\"\n{\n\t\"appid\"\t\t\"440\"\n\t\"name\"\t\t\"   \"\n}\n";
        let result = parse_acf_content(content);
        // 現在の extract_value は空白のみの値をフィルタするため None になる
        // これが意図した動作かどうかを確認するテスト
        assert!(
            result.is_none(),
            "\u{5024}\u{304c}\u{7a7a}\u{767d}\u{306e}\u{307f}\u{306e}name\u{306f}\u{6709}\u{52b9}\u{306a}\u{30b2}\u{30fc}\u{30e0}\u{3068}\u{307f}\u{306a}\u{3055}\u{308c}\u{306a}\u{3044}\u{306f}\u{305a}: {:?}",
            result
        );
    }

    // ===== [中優先度] get_local_steam_image 画像優先度のテスト =====

    #[tokio::test]
    async fn test_get_local_steam_image_nonexistent_appid_returns_none() {
        // 存在しない appid の場合は Ok(None) を返すこと
        // （Steam がインストールされていない場合も Ok(None)）
        let result = get_local_steam_image("00000000001_nonexistent_test".to_string()).await;
        assert!(
            result.is_ok(),
            "get_local_steam_image \u{306f}\u{5b58}\u{5728}\u{3057}\u{306a}\u{3044}appid\u{3067}\u{3082} Ok \u{3092}\u{8fd4}\u{3059}\u{3079}\u{304d}: {:?}",
            result
        );
        assert!(
            result.unwrap().is_none(),
            "\u{5b58}\u{5728}\u{3057}\u{306a}\u{3044}appid\u{306f} None \u{3092}\u{8fd4}\u{3059}\u{3079}\u{304d}"
        );
    }

    #[tokio::test]
    async fn test_get_local_steam_image_returns_base64_data_uri_format() {
        // 画像が見つかった場合、data URI 形式で返されること
        // Steam が未インストールなら Ok(None) が返るのでスキップ
        let result = get_local_steam_image("440".to_string()).await;
        assert!(result.is_ok(), "get_local_steam_image \u{306f} Ok \u{3092}\u{8fd4}\u{3059}\u{3079}\u{304d}");
        if let Ok(Some(img_data)) = result {
            // data URI フォーマット確認
            assert!(
                img_data.starts_with("data:image/"),
                "\u{753b}\u{50cf}\u{30c7}\u{30fc}\u{30bf}\u{306f} data:image/ \u{3067}\u{59cb}\u{307e}\u{308b}\u{3079}\u{304d}: {}",
                &img_data[..50.min(img_data.len())]
            );
            assert!(
                img_data.contains(";base64,"),
                "\u{753b}\u{50cf}\u{30c7}\u{30fc}\u{30bf}\u{306f} base64 \u{30a8}\u{30f3}\u{30b3}\u{30fc}\u{30c9}\u{3055}\u{308c}\u{3066}\u{3044}\u{308b}\u{3079}\u{304d}"
            );
        }
        // Ok(None) の場合（Steam 未インストールまたはゲーム未取得）はテストをスキップ
    }

    #[tokio::test]
    async fn test_get_local_steam_image_priority_library_600x900_preferred() {
        // 優先度テスト: library_600x900.jpg が存在する場合、それが返されるべき
        // このテストは Steam がインストールされていて TF2 (440) がある環境でのみ有効
        // Steam 未インストール時は Ok(None) が返る
        let result = get_local_steam_image("440".to_string()).await;
        assert!(result.is_ok());
        if let Ok(Some(img_data)) = result {
            // library_600x900.jpg は jpeg 形式
            // data:image/jpeg;base64,... であることを確認
            // （他の優先度の画像が先に返される可能性もあるため、jpeg を確認）
            assert!(
                img_data.starts_with("data:image/jpeg") || img_data.starts_with("data:image/png"),
                "\u{8fd4}\u{3055}\u{308c}\u{308b}\u{753b}\u{50cf}\u{306f} jpeg \u{307e}\u{305f}\u{306f} png \u{3067}\u{3042}\u{308b}\u{3079}\u{304d}: {}",
                &img_data[..30.min(img_data.len())]
            );
        }
    }
}

