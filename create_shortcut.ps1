$WshShell = New-Object -ComObject WScript.Shell
$DesktopPath = [System.IO.Path]::Combine($env:USERPROFILE, "Desktop")
$ShortcutPath = [System.IO.Path]::Combine($DesktopPath, "音声出力切り替え.lnk")
$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = "c:\自作プロジェクト\音声出力切り替え\start_app.bat"
$Shortcut.WorkingDirectory = "c:\自作プロジェクト\音声出力切り替え"
$Shortcut.Save()
