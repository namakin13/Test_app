using System;
using System.Collections.Generic;
using System.Windows;
using System.Windows.Controls;
using System.Diagnostics;
using SteamLauncher.Backend;

namespace SteamLauncher
{
    /// <summary>
    /// MainWindow.xaml の相互作用ロジック
    /// </summary>
    public partial class MainWindow : Window
    {
        private readonly SteamManager _steamManager;

        public MainWindow()
        {
            InitializeComponent();

            // SteamManager のインスタンスを作成
            _steamManager = new SteamManager();

            // 画面読み込み時にゲームリストを表示
            LoadGames();
        }

        /// <summary>
        /// Steam からインストール済みのゲームを取得し、UI に表示します。
        /// </summary>
        private void LoadGames()
        {
            try
            {
                // バックエンドのロジックを呼び出し
                List<SteamManager.SteamGame> games = _steamManager.GetInstalledGames();

                // UI の ItemsControl にデータをセット
                GamesList.ItemsSource = games;

                if (games.Count == 0)
                {
                    MessageBox.Show("インストール済みの Steam ゲームが見つかりませんでした。");
                }
            }
            catch (Exception ex)
            {
                MessageBox.Show($"ゲームリストの取得中にエラーが発生しました: {ex.Message}");
            }
        }

        /// <summary>
        /// 「起動」ボタンがクリックされた時の処理
        /// </summary>
        private void OnLaunchButtonClick(object sender, RoutedEventArgs e)
        {
            if (sender is Button button && button.Tag is string appid)
            {
                // ボタンの Tag プロパティに保存しておいた AppID を使って起動
                _steamManager.LaunchGame(appid);
            }
        }

        /// <summary>
        /// 画像の読み込みに失敗した時の処理
        /// </summary>
        private void OnImageFailed(object sender, ExceptionRoutedEventArgs e)
        {
            if (sender is Image image)
            {
                // 読み込みに失敗した場合は画像を非表示にする（または代替画像を表示する）
                image.Visibility = Visibility.Collapsed;
                Debug.WriteLine($"Image load failed: {e.ErrorException.Message}");
            }
        }
    }
}
