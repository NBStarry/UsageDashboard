package app.usagedashboard

import android.content.Context
import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  // Rust(usage_dashboard_lib)导出的初始化:把 Context 交给 ndk_context,
  // 供命令层 JNI(添加小组件)取 context。库由 TauriActivity 加载。
  external fun nativeInit(context: Context)

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    try { nativeInit(applicationContext) } catch (e: Throwable) { }
  }

  // 离开 App 时,把 Rust 在前台刷新里写好的最新快照刷进主屏小组件。
  override fun onStop() {
    super.onStop()
    UsageWidgetProvider.refreshAll(this)
    UsageDoubleWidgetProvider.refreshAll(this)
  }
}
