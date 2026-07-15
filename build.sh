#!/bin/bash
# build.sh — 构建 水利工具箱 HydroMac(原生 SwiftUI + vendored Rust hydro-cli 后端)。
#
#   ./build.sh             构建 → ./build/水利工具箱.app
#   ./build.sh --install   构建 + 装入 /Applications(失败退 ~/Applications)
#
# 形态 = 裸 SwiftUI 源码(Sources/*.swift,swiftc 直编)+ cargo 编 cli/ → hydro-cli 嵌 .app Resources,自包含。
# adhoc 签名(无需付费开发者证书);若使用下载的 zip 而非本地构建,先清 quarantine:
#   xattr -cr "/Applications/水利工具箱.app"
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

APP_NAME="HydroMac"          # CFBundleName/可执行名(open -a HydroMac)
DISPLAY_NAME="水利工具箱"      # 安装文件名 = 显示名
BUNDLE_ID="io.github.zengtianli.HydroMac"

if [ -d /Applications/Xcode.app ]; then
  export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
fi

echo "→ cargo 编 Rust 后端 hydro-cli(release)…"
( cd cli && cargo build --release )
CLI_BIN="cli/target/release/hydro-cli"
[ -x "$CLI_BIN" ] || { echo "✗ hydro-cli 未构建出"; exit 1; }

echo "→ swiftc 编译 SwiftUI 前端(release)…"
mkdir -p build
xcrun swiftc -O -parse-as-library -target arm64-apple-macosx14.0 \
  Sources/Models.swift \
  Sources/CommandPalette.swift \
  Sources/BackendClient.swift \
  Sources/DataTableView.swift \
  Sources/AnnualView.swift \
  Sources/CapacityView.swift \
  Sources/EfficiencyView.swift \
  Sources/ReservoirView.swift \
  Sources/DistrictView.swift \
  Sources/IrrigationView.swift \
  Sources/RainfallView.swift \
  Sources/GeocodeView.swift \
  Sources/ContentView.swift \
  Sources/HydroMacApp.swift \
  -o "build/$APP_NAME"

echo "→ 打包 .app bundle(嵌 hydro-cli 进 Resources)…"
APP="build/$DISPLAY_NAME.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "build/$APP_NAME" "$APP/Contents/MacOS/$APP_NAME"
cp "$CLI_BIN" "$APP/Contents/Resources/hydro-cli"        # 后端随 app 走,自包含
cp "$DIR/icon/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key><string>$APP_NAME</string>
	<key>CFBundleName</key><string>$APP_NAME</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
	<key>CFBundleShortVersionString</key><string>1.0.0</string>
	<key>LSMinimumSystemVersion</key><string>14.0</string>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSPrincipalClass</key><string>NSApplication</string>
</dict>
</plist>
PLIST

echo "→ post-build:注入 DisplayName/BundleID/Icon/Version…"
plutil -replace CFBundleDisplayName -string "$DISPLAY_NAME" "$APP/Contents/Info.plist"
plutil -replace CFBundleIdentifier  -string "$BUNDLE_ID"    "$APP/Contents/Info.plist"
plutil -replace CFBundleIconFile    -string "AppIcon"       "$APP/Contents/Info.plist"
plutil -replace CFBundleVersion     -string "$(git -C "$DIR" rev-list --count HEAD 2>/dev/null || echo 1)" "$APP/Contents/Info.plist"

codesign --force -s - "$APP"
echo "✅ 构建完成 → $APP"

if [ "${1:-}" = "--install" ]; then
  pkill -x "$APP_NAME" 2>/dev/null || true
  DEST="/Applications/$DISPLAY_NAME.app"
  if ! rm -rf "$DEST" 2>/dev/null || ! cp -R "$APP" "$DEST" 2>/dev/null; then
    DEST="$HOME/Applications/$DISPLAY_NAME.app"
    mkdir -p "$HOME/Applications"
    rm -rf "$DEST"; cp -R "$APP" "$DEST"
  fi
  echo "✅ 已安装 → $DEST"
fi
