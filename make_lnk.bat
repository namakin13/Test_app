@echo off
powershell -Command "$ws=New-Object -ComObject WScript.Shell;$s=$ws.CreateShortcut($env:USERPROFILE+'\Desktop\AudioSwitcher.lnk');$s.TargetPath='%~dp0start_app.bat';$s.WorkingDirectory='%~dp0';$s.Save()"
