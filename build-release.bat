@echo off
chcp 65001 >nul
title "Project Build (Profile:Release)"

echo "crates.io의 최신 정보를 가져옵니다."
cargo update

echo "프로젝트를 빌드합니다."
cargo build --release

set target="\"%cd%\target\release\server.exe\""
set shortcutPath="\"%cd%\server.lnk"\"
set description="\"Server Shortcut"\"

rem PowerShell을 사용하여 바로가기를 생성합니다.
powershell -Command "$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut(%shortcutPath%); $Shortcut.TargetPath = %target%; $Shortcut.Description = %description%; $Shortcut.Save()"
echo "프로젝트 디렉토리에 server 바로가기가 생성되었습니다."


set target="\"%cd%\target\release\client.exe\""
set shortcutPath="\"%cd%\client.lnk"\"
set description="\"Client Shortcut"\"

rem PowerShell을 사용하여 바로가기를 생성합니다.
powershell -Command "$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut(%shortcutPath%); $Shortcut.TargetPath = %target%; $Shortcut.Description = %description%; $Shortcut.Save()"
echo "프로젝트 디렉토리에 client 바로가기가 생성되었습니다."

pause
