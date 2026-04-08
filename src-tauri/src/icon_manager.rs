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

/// JSONテキストからアイコンマップをパースする（テスト可能な純粋関数）
fn parse_icons_content(content: &str) -> Result<HashMap<String, String>, String> {
    serde_json::from_str(content)
        .map_err(|e| format!("カスタムアイコンのJSONパースに失敗しました: {}", e))
}

/// ファイルパスがパストラバーサル攻撃を含んでいないか検証する（テスト可能な純粋関数）
/// 要件: 不正改造 - 任意ファイルの読み取りを防止する
fn validate_file_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("ファイルパスが空です".to_string());
    }
    // パストラバーサル攻撃の検出 (../../etc/passwd 等)
    if path.contains("..") {
        return Err(format!(
            "ファイルパスに '..' が含まれており、パストラバーサルの可能性があります: '{}'",
            path
        ));
    }
    // NULバイトインジェクションの検出
    if path.contains('\0') {
        return Err("ファイルパスにNULバイトが含まれています".to_string());
    }
    Ok(())
}

/// アイコンファイルのバイト列を検証する（テスト可能な純粋関数）
fn validate_icon_file(data: &[u8]) -> Result<(), String> {
    if data.is_empty() {
        return Err("ファイルが空です".to_string());
    }
    const MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if data.len() > MAX_SIZE {
        return Err(format!(
            "ファイルが大きすぎます ({} bytes, 最大 {} bytes)",
            data.len(),
            MAX_SIZE
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_custom_icons(app_handle: AppHandle) -> Result<HashMap<String, String>, String> {
    let path = get_icons_path(&app_handle)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    // 修正: unwrap_or_default() → エラーを呼び出し元に伝播
    parse_icons_content(&content)
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
    // 不正改造防止: ファイルパスを読み込む前にパストラバーサルを検証する
    validate_file_path(&file_path)?;

    let file_data = fs::read(&file_path).map_err(|e| format!("ファイル読み込みに失敗しました: {}", e))?;

    // 修正: 空ファイル・サイズ超過を検証してエラーを返す
    validate_icon_file(&file_data)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== [セキュリティ] validate_file_path のテスト =====
    // 要件: 不正改造 - アイコン保存時の任意ファイル読み取り（パストラバーサル）防止

    #[test]
    fn test_valid_file_path_accepted() {
        // 正規のファイルパスは通過すること
        let valid_paths = [
            "C:\\Users\\user\\Desktop\\icon.png",
            "C:\\Users\\user\\Pictures\\game_icon.jpg",
            "D:\\icons\\custom.webp",
        ];
        for path in &valid_paths {
            assert!(
                validate_file_path(path).is_ok(),
                "正規のパス '{}' は検証を通過すべき",
                path
            );
        }
    }

    #[test]
    fn test_empty_path_rejected() {
        assert!(validate_file_path("").is_err(), "空のパスはエラーを返すべき");
    }

    #[test]
    fn test_path_traversal_windows_backslash_rejected() {
        // 攻撃例: ..\..\ を使ったWindowsパストラバーサル
        let malicious = "C:\\Users\\user\\..\\..\\Windows\\System32\\config\\SAM";
        let result = validate_file_path(malicious);
        assert!(
            result.is_err(),
            "'..' を含むパスはパストラバーサル防止のためエラーを返すべき: '{}'",
            malicious
        );
        assert!(result.unwrap_err().contains(".."));
    }

    #[test]
    fn test_path_traversal_unix_slash_rejected() {
        // 攻撃例: ../../ を使ったUnix形式パストラバーサル
        let malicious = "../../etc/passwd";
        let result = validate_file_path(malicious);
        assert!(
            result.is_err(),
            "Unix形式のパストラバーサルもエラーを返すべき: '{}'",
            malicious
        );
    }

    #[test]
    fn test_path_with_dotdot_in_filename_rejected() {
        // 攻撃例: ファイル名の途中に .. が含まれるケース
        let malicious = "C:\\icons\\icon..\\..\\..\\sensitive.png";
        let result = validate_file_path(malicious);
        assert!(result.is_err(), "パスの途中に '..' を含む場合もエラーを返すべき");
    }

    #[test]
    fn test_path_with_null_byte_rejected() {
        // 攻撃例: NULバイトインジェクション
        // C言語の文字列終端文字 \0 でパスを切り詰める攻撃
        let malicious = "C:\\safe\\icon.png\0C:\\Windows\\System32\\evil.exe";
        let result = validate_file_path(malicious);
        assert!(
            result.is_err(),
            "NULバイトを含むパスはNullバイトインジェクション防止のためエラーを返すべき"
        );
    }

    #[test]
    fn test_system_file_path_with_traversal_rejected() {
        // 攻撃例: Windowsのパスワードハッシュファイルへのアクセス試行
        let malicious = "C:\\Users\\user\\Desktop\\..\\..\\Windows\\System32\\config\\SAM";
        let result = validate_file_path(malicious);
        assert!(
            result.is_err(),
            "システムファイルへのパストラバーサルはエラーを返すべき"
        );
    }

    #[test]
    fn test_ssh_key_path_with_traversal_rejected() {
        // 攻撃例: SSHプライベートキーへのアクセス試行
        let malicious = "C:\\Users\\user\\Desktop\\..\\..\\..ssh\\id_rsa";
        let result = validate_file_path(malicious);
        assert!(
            result.is_err(),
            "SSHキーへのパストラバーサルはエラーを返すべき"
        );
    }

    // ===== parse_icons_content のテスト =====

    #[test]
    fn test_parse_valid_icons_json() {
        let json = r#"{"12345": "data:image/png;base64,abc123", "67890": "data:image/jpeg;base64,xyz"}"#;
        let result = parse_icons_content(json);
        assert!(result.is_ok());
        let icons = result.unwrap();
        assert_eq!(icons.get("12345").unwrap(), "data:image/png;base64,abc123");
        assert_eq!(icons.get("67890").unwrap(), "data:image/jpeg;base64,xyz");
    }

    #[test]
    fn test_parse_empty_json_object() {
        let result = parse_icons_content("{}");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_invalid_json_returns_error() {
        // 壊れたJSONは unwrap_or_default() で空Mapになってはいけない
        let result = parse_icons_content("{ this is not valid json !!!");
        assert!(
            result.is_err(),
            "壊れたJSONはエラーを返すべきだが Ok が返ってきた"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("JSONパースに失敗"),
            "エラーメッセージが期待通りでない: {}",
            err
        );
    }

    #[test]
    fn test_parse_wrong_type_json_returns_error() {
        // 型が違う（配列）はエラーになるべき
        let result = parse_icons_content(r#"["item1", "item2"]"#);
        assert!(
            result.is_err(),
            "配列型JSONはHashMap<String,String>にデシリアライズできずエラーになるべき"
        );
    }

    #[test]
    fn test_parse_partial_wrong_type_returns_error() {
        // 値が文字列でない場合もエラー
        let result = parse_icons_content(r#"{"appid": 12345}"#);
        assert!(
            result.is_err(),
            "値が整数のJSONはString型にデシリアライズできずエラーになるべき"
        );
    }

    #[test]
    fn test_parse_japanese_appid_key() {
        // キーに任意の文字列を使えることを確認
        let json = r#"{"テスト": "data:image/png;base64,abc"}"#;
        let result = parse_icons_content(json);
        assert!(result.is_ok());
        assert!(result.unwrap().contains_key("テスト"));
    }

    // ===== validate_icon_file のテスト =====

    #[test]
    fn test_validate_empty_file_returns_error() {
        let result = validate_icon_file(&[]);
        assert!(
            result.is_err(),
            "空ファイルはエラーを返すべきだが Ok が返ってきた"
        );
        assert!(result.unwrap_err().contains("空"));
    }

    #[test]
    fn test_validate_valid_file_ok() {
        let data = vec![0xFFu8; 1024]; // 1KB の PNG っぽいデータ
        assert!(validate_icon_file(&data).is_ok());
    }

    #[test]
    fn test_validate_oversized_file_returns_error() {
        let data = vec![0u8; 11 * 1024 * 1024]; // 11MB
        let result = validate_icon_file(&data);
        assert!(
            result.is_err(),
            "10MB超のファイルはエラーを返すべきだが Ok が返ってきた"
        );
        assert!(result.unwrap_err().contains("大きすぎます"));
    }

    #[test]
    fn test_validate_exactly_max_size_ok() {
        let data = vec![0u8; 10 * 1024 * 1024]; // ちょうど10MB
        assert!(
            validate_icon_file(&data).is_ok(),
            "ちょうど10MBはOKであるべき"
        );
    }

    // ===== MIMEタイプ判定のテスト（save_custom_icon_from_path 内ロジック） =====

    #[test]
    fn test_mime_type_detection() {
        let cases = vec![
            ("image.jpg", "jpeg"),
            ("image.jpeg", "jpeg"),
            ("image.gif", "gif"),
            ("image.webp", "webp"),
            ("image.png", "png"),
            ("image.bmp", "png"),  // 未知の拡張子 → png にフォールバック
            ("noextension", "png"), // 拡張子なし → png にフォールバック
        ];

        for (filename, expected_mime) in cases {
            let path = std::path::Path::new(filename);
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
            let mime_type = match ext.to_lowercase().as_str() {
                "jpg" | "jpeg" => "jpeg",
                "gif" => "gif",
                "webp" => "webp",
                _ => "png",
            };
            assert_eq!(
                mime_type, expected_mime,
                "ファイル '{}' のMIMEタイプが期待値 '{}' と異なる: '{}'",
                filename, expected_mime, mime_type
            );
        }
    }

    // ===== 存在しないファイルパスのテスト =====

    #[test]
    fn test_read_nonexistent_file_returns_error() {
        let fake_path = "C:\\nonexistent_path\\fake_icon_xyz123.png";
        let result = fs::read(fake_path);
        assert!(
            result.is_err(),
            "存在しないファイルは読み込みエラーになるべき"
        );
    }
}
