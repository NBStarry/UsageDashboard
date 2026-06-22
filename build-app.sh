#!/bin/bash
# 编译并组装 UsageBar.app(菜单栏 App:无 Dock 图标 + 开机自启 + ad-hoc 签名)。
set -euo pipefail

cd "$(dirname "$0")"

APP_NAME="UsageBar"
BUNDLE_ID="${BUNDLE_ID:-app.usagebar.menu}"
VERSION="1.0.0"
APP="${APP_NAME}.app"

echo "==> swift build -c release"
swift build -c release

echo "==> 组装 ${APP}"
rm -rf "${APP}"
mkdir -p "${APP}/Contents/MacOS"
mkdir -p "${APP}/Contents/Resources"

cp ".build/release/${APP_NAME}" "${APP}/Contents/MacOS/${APP_NAME}"

# App 图标(若存在 AppIcon.icns 则嵌入)
ICON_LINE=""
if [ -f "AppIcon.icns" ]; then
    cp "AppIcon.icns" "${APP}/Contents/Resources/AppIcon.icns"
    ICON_LINE='    <key>CFBundleIconFile</key>        <string>AppIcon</string>'
fi

cat > "${APP}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>            <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>     <string>用量看板</string>
    <key>CFBundleIdentifier</key>      <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>         <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key> <string>${VERSION}</string>
    <key>CFBundleExecutable</key>      <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>     <string>APPL</string>
    <key>LSMinimumSystemVersion</key>  <string>13.0</string>
    <key>LSUIElement</key>             <true/>
${ICON_LINE}
    <key>NSHumanReadableCopyright</key><string>UsageBar</string>
</dict>
</plist>
PLIST

echo "==> ad-hoc 签名"
codesign --force --deep --sign - "${APP}"

echo "==> 完成: $(pwd)/${APP}"
echo "    运行:  open ${APP}"
echo "    安装:  cp -r ${APP} /Applications/  (开机自启需 App 在稳定路径)"
