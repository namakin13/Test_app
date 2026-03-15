using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using Microsoft.Win32;

namespace SteamLauncher.Backend
{
    /// <summary>
    /// Steam のインストール情報を管理し、ゲームのリスト取得や起動を行うクラスです。
    /// </summary>
    public class SteamManager
    {
        /// <summary>
        /// Steam ゲームの情報を保持するクラスです。
        /// </summary>
        public class SteamGame
        {
            public string Name { get; set; }
            public string AppId { get; set; }
            // AppID から Steam CDN のバナー画像 URL を生成
            public string HeaderImageUrl => $"https://cdn.akamai.steamstatic.com/steam/apps/{AppId}/header.jpg";

            public override string ToString() => $"{Name} (ID: {AppId})";
        }

        /// <summary>
        /// Windows レジストリから Steam のインストールパスを取得します。
        /// </summary>
        /// <returns>インストールパス。取得できない場合は null を返します。</returns>
        public string GetSteamInstallPath()
        {
            // Windows レジストリの HKEY_CURRENT_USER\SOFTWARE\Valve\Steam キーを参照します
            string registryPath = @"SOFTWARE\Valve\Steam";
            using (RegistryKey key = Registry.CurrentUser.OpenSubKey(registryPath))
            {
                if (key != null)
                {
                    // "SteamPath" の値を取得します (例: C:/Program Files (x86)/Steam)
                    return key.GetValue("SteamPath")?.ToString();
                }
            }
            return null;
        }

        /// <summary>
        /// インストール済みの Steam ゲーム（name と appid）のリストを取得します。
        /// </summary>
        /// <returns>SteamGame オブジェクトのリスト</returns>
        public List<SteamGame> GetInstalledGames()
        {
            List<SteamGame> games = new List<SteamGame>();
            string steamPath = GetSteamInstallPath();

            if (string.IsNullOrEmpty(steamPath))
            {
                return games;
            }

            // Steam のメインライブラリフォルダ: steamapps
            string steamAppsPath = Path.Combine(steamPath, "steamapps");

            if (!Directory.Exists(steamAppsPath))
            {
                return games;
            }

            // steamapps フォルダ内の すべての .acf ファイルを検索します
            // 各 .acf ファイルには 1 つのゲームの情報が記述されています
            string[] acfFiles = Directory.GetFiles(steamAppsPath, "*.acf");

            foreach (string file in acfFiles)
            {
                var gameInfo = ParseAcfFile(file);
                if (gameInfo != null)
                {
                    games.Add(gameInfo);
                }
            }

            return games;
        }

        /// <summary>
        /// .acf ファイルを解析してゲーム情報を抽出します。
        /// VDF (Valve Data Format) 形式を簡易的に解析します。
        /// </summary>
        private SteamGame ParseAcfFile(string filePath)
        {
            try
            {
                string name = null;
                string appid = null;

                // ファイルを 1 行ずつ読み込みます
                foreach (string line in File.ReadAllLines(filePath))
                {
                    string trimmed = line.Trim();

                    // "name" "GameTitle" の形式を簡易的に抽出
                    if (trimmed.StartsWith("\"name\"", StringComparison.OrdinalIgnoreCase))
                    {
                        name = ExtractValue(trimmed);
                    }
                    // "appid" "12345" の形式を抽出
                    else if (trimmed.StartsWith("\"appid\"", StringComparison.OrdinalIgnoreCase))
                    {
                        appid = ExtractValue(trimmed);
                    }

                    // 両方見つかったら早期終了
                    if (name != null && appid != null)
                    {
                        return new SteamGame { Name = name, AppId = appid };
                    }
                }
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"Error parsing {filePath}: {ex.Message}");
            }

            return null;
        }

        /// <summary>
        /// クォーテーションで囲まれた値の部分を抽出します。
        /// </summary>
        private string ExtractValue(string line)
        {
            // 列挙された文字列の最後の方が値なので、最後から引用符で分割します
            string[] parts = line.Split(new[] { '\"' }, StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length >= 2)
            {
                return parts.Last();
            }
            return null;
        }

        /// <summary>
        /// URI スキームを使用してゲームを起動します。
        /// </summary>
        /// <param name="appid">起動したいゲームの AppID</param>
        public void LaunchGame(string appid)
        {
            if (string.IsNullOrEmpty(appid)) return;

            // steam://run/APPID という形式の URL を実行すると、Steam 経由でゲームが起動します
            string url = $"steam://run/{appid}";

            try
            {
                // Windows で URL を開くための標準的な方法
                Process.Start(new ProcessStartInfo
                {
                    FileName = url,
                    UseShellExecute = true // URI スキームを正しく処理するために必要
                });
            }
            catch (Exception ex)
            {
                Debug.WriteLine($"Failed to launch game: {ex.Message}");
            }
        }
    }
}
