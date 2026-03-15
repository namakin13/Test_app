Set oWS = WScript.CreateObject("WScript.Shell")
sLinkFile = oWS.ExpandEnvironmentStrings("%USERPROFILE%\Desktop\音声出力切り替え.lnk")
Set oLink = oWS.CreateShortcut(sLinkFile)
oLink.TargetPath = "c:\自作プロジェクト\音声出力切り替え\start_app.bat"
oLink.WorkingDirectory = "c:\自作プロジェクト\音声出力切り替え"
oLink.Save
