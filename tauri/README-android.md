# UsageDashboard — Android (Tauri mobile)

Android 端基于 Tauri 2 移动端，复用 `tauri/` 下的 Svelte 前端 + Rust 后端。
桌面专属代码（托盘 / autostart / popover 窗口）已用 `#[cfg(desktop)]` 隔离，
移动端全屏显示主 App。本文档记录在一台新机器上从零把 Android 构建跑起来的步骤。

> 现状：`tauri android build --debug --target aarch64` 已验证能编译并产出 APK。
> 真机/模拟器实际运行与移动端 UI 适配（隐藏 quit 按钮等）尚未完成。

## 1. 工具链前置

- **Rust**（rustup）+ Android 编译目标：
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  . "$HOME/.cargo/env"
  rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
  ```
- **JDK 17**（Android Gradle 需要）。
- **Android SDK + NDK**。命令行方式（macOS，Homebrew）：
  ```bash
  brew install --cask android-commandlinetools
  export ANDROID_HOME="/opt/homebrew/share/android-commandlinetools"
  yes | sdkmanager --licenses
  sdkmanager "platform-tools" "platforms;android-35" "build-tools;35.0.1" "ndk;27.2.12479018"
  ```
  （也可装 Android Studio，用其自带 SDK Manager 装同样的 platform-35 / build-tools / NDK r27c。）

- **环境变量**（写进 `~/.zprofile` 或 `~/.zshenv`，按实际 NDK 版本号调整）：
  ```bash
  export ANDROID_HOME="/opt/homebrew/share/android-commandlinetools"   # 或 Android Studio 的 sdk 路径
  export NDK_HOME="$ANDROID_HOME/ndk/27.2.12479018"
  export JAVA_HOME="$(/usr/libexec/java_home -v 17)"
  export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin:$HOME/.cargo/bin:$PATH"
  ```

## 2. 国内网络（GFW）修复 —— 否则 Gradle 必失败

Gradle（JVM）**不读** `http_proxy` 环境变量，会直连 `dl.google.com` /
`repo.maven.apache.org` 被 GFW 掐断 TLS（报 `Remote host terminated the handshake`）。
两处全局配置（都放 `~/.gradle/`，不随项目 re-init 丢失）：

`~/.gradle/gradle.properties`（代理端口按你的 Clash/代理实际端口）：
```properties
systemProp.http.proxyHost=127.0.0.1
systemProp.http.proxyPort=7897
systemProp.https.proxyHost=127.0.0.1
systemProp.https.proxyPort=7897
systemProp.http.nonProxyHosts=localhost|127.0.0.1
```

`~/.gradle/init.gradle`（阿里云镜像，国内直连稳定，避免国际仓握手）：
```groovy
def aliyun = [
    'https://maven.aliyun.com/repository/public',
    'https://maven.aliyun.com/repository/google',
    'https://maven.aliyun.com/repository/gradle-plugin',
    'https://maven.aliyun.com/repository/central',
]
allprojects {
    buildscript { repositories { aliyun.each { u -> maven { url u } } } }
    repositories { aliyun.each { u -> maven { url u } } }
}
settingsEvaluated { settings ->
    settings.pluginManagement { repositories { aliyun.each { u -> maven { url u } }; gradlePluginPortal() } }
    try { settings.dependencyResolutionManagement { repositories { aliyun.each { u -> maven { url u } } } } } catch (ignored) {}
}
```

（境外网络畅通的机器可跳过本节。）

## 3. 构建

```bash
cd tauri
npm install
chmod +x node_modules/.bin/*            # 跨平台 checkout 后 .bin 常丢执行权限

# gen/android 已随仓库提交;若缺失或换了 identifier 才需要重新生成:
# npx tauri android init

# 编译 + 打包调试 APK(单 ABI 最快)
npx tauri android build --debug --target aarch64
# 产物: src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk

# 接真机(开 USB 调试)后实机运行 + 热重载:
# npx tauri android dev
```

构建相关常量：identifier/applicationId = `app.usagedashboard`；
compile SDK 35；NDK r27c（27.2.12479018）。

## 4. 待办（接着做）

- 真机/模拟器实际运行，验证 Svelte UI 在手机渲染。
- 移动端 UI 适配：隐藏 quit 按钮、安全区适配。
- Phase 2：手机端凭证（先通 PhanRouter `save_new_api_credentials` app 内输入 → 看到真实用量）。
- Phase 3：Android 原生主屏小组件（App Widget，读共享存储中的用量快照）。
