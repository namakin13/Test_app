Option Explicit
Dim oWS, sLinkFile, oLink, sName
Set oWS = CreateObject("WScript.Shell")
' "音声出力切り替え"
sName = ChrW(38899) & ChrW(22768) & ChrW(20986) & ChrW(21147) & ChrW(20999) & ChrW(12426) & ChrW(26367) & ChrW(12360)
sLinkFile = oWS.ExpandEnvironmentStrings("%USERPROFILE%\Desktop\") & sName & ".lnk"
On Error Resume Next
Set oLink = oWS.CreateShortcut(sLinkFile)
oLink.TargetPath = "c:\自作プロジェクト\音声出力切り替え\start_app.bat"
oLink.WorkingDirectory = "c:\自作プロジェクト\音声出力切り替え"
oLink.Save
If Err.Number <> 0 Then
    WScript.Echo "Error: " & Err.Description
End If
