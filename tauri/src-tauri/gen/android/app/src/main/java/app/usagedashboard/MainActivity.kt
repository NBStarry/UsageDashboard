package app.usagedashboard

import android.os.Bundle
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  // 离开 App 时,把 Rust 在前台刷新里写好的最新快照刷进主屏小组件。
  override fun onStop() {
    super.onStop()
    UsageWidgetProvider.refreshAll(this)
  }
}
