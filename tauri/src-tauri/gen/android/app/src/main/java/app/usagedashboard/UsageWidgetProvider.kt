package app.usagedashboard

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.graphics.Color
import android.view.View
import android.widget.RemoteViews
import org.json.JSONObject
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// 主屏小组件:读 Rust 写出的 dataDir/widget.json,渲染前 3 个服务的两行用量。
// 数据由 app 进程在每次 refresh 后写入;MainActivity.onStop() 调 refreshAll() 触发重绘,
// 另有 30 分钟系统周期更新兜底。点击整体打开 App。
class UsageWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(context: Context, mgr: AppWidgetManager, ids: IntArray) {
        for (id in ids) render(context, mgr, id)
    }

    companion object {
        // 供 MainActivity 在离开 App 时调用,把刚写的最新快照刷进小组件。
        fun refreshAll(context: Context) {
            val mgr = AppWidgetManager.getInstance(context)
            val cn = ComponentName(context, UsageWidgetProvider::class.java)
            for (id in mgr.getAppWidgetIds(cn)) render(context, mgr, id)
        }

        private val ROWS = intArrayOf(R.id.w_row0, R.id.w_row1, R.id.w_row2)
        private val DOTS = intArrayOf(R.id.w_dot0, R.id.w_dot1, R.id.w_dot2)
        private val TITLES = intArrayOf(R.id.w_title0, R.id.w_title1, R.id.w_title2)
        private val L1S = intArrayOf(R.id.w_l1_0, R.id.w_l1_1, R.id.w_l1_2)
        private val L2S = intArrayOf(R.id.w_l2_0, R.id.w_l2_1, R.id.w_l2_2)

        private fun render(context: Context, mgr: AppWidgetManager, id: Int) {
            val views = RemoteViews(context.packageName, R.layout.usage_widget)

            // 点击整体打开 App。
            val launch = context.packageManager.getLaunchIntentForPackage(context.packageName)
            if (launch != null) {
                val flags = PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
                val pi = PendingIntent.getActivity(context, 0, launch, flags)
                views.setOnClickPendingIntent(R.id.w_root, pi)
            }

            try {
                val f = File(context.dataDir, "widget.json")
                if (!f.exists()) {
                    views.setTextViewText(R.id.w_updated, "打开 App 加载数据")
                    for (i in 0 until 3) views.setViewVisibility(ROWS[i], View.GONE)
                    mgr.updateAppWidget(id, views)
                    return
                }
                val obj = JSONObject(f.readText())
                val ms = obj.optLong("updatedAtMs", 0L)
                val time = if (ms > 0) SimpleDateFormat("HH:mm", Locale.getDefault()).format(Date(ms)) else "--:--"
                views.setTextViewText(R.id.w_updated, "更新于 $time")

                val arr = obj.optJSONArray("services")
                val n = arr?.length() ?: 0
                for (i in 0 until 3) {
                    if (arr != null && i < n) {
                        val s = arr.getJSONObject(i)
                        views.setViewVisibility(ROWS[i], View.VISIBLE)
                        views.setTextViewText(TITLES[i], s.optString("title"))
                        views.setTextViewText(L1S[i], s.optString("line1"))
                        val l2 = s.optString("line2")
                        views.setTextViewText(L2S[i], l2)
                        views.setViewVisibility(L2S[i], if (l2.isEmpty()) View.GONE else View.VISIBLE)
                        val color = try { Color.parseColor(s.optString("accent", "#8E8E93")) } catch (e: Exception) { Color.GRAY }
                        views.setTextColor(DOTS[i], color)
                    } else {
                        views.setViewVisibility(ROWS[i], View.GONE)
                    }
                }
            } catch (e: Exception) {
                views.setTextViewText(R.id.w_updated, "读取失败")
                for (i in 0 until 3) views.setViewVisibility(ROWS[i], View.GONE)
            }
            mgr.updateAppWidget(id, views)
        }
    }
}
