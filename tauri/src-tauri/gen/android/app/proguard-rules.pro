# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile

# TokenDeck: 保留全部 app 类不被混淆/裁剪。
# 原因:
#  - MainActivity.nativeInit 由 Rust 用 JNI 符号名 Java_app_usagedashboard_MainActivity_nativeInit
#    按名查找,类/方法名被改名就找不到 -> ndk_context 不初始化 -> 首启 panic。
#  - WebLoginActivity 由 action intent 字符串 app.usagedashboard.LOGIN_NEWAPI 启动,
#    UsageWidgetProvider/WidgetConfigActivity 经 manifest+SharedPreferences 反射式使用。
-keep class app.usagedashboard.** { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}
