package app.usagedashboard

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.res.ColorStateList
import android.graphics.Color
import android.os.Build
import android.view.View
import android.widget.RemoteViews
import org.json.JSONObject
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// 单服务主屏小组件:每个实例绑定一个渠道(WidgetConfigActivity 选定,存 SharedPreferences),
// 读 Rust 写的 dataDir/widget.json,按该渠道渲染窗口进度条 / 余额。点击打开 App。
class UsageWidgetProvider : AppWidgetProvider() {
    override fun onUpdate(context: Context, mgr: AppWidgetManager, ids: IntArray) {
        for (id in ids) renderOne(context, mgr, id)
    }

    override fun onDeleted(context: Context, ids: IntArray) {
        val p = context.getSharedPreferences("usage_widget", Context.MODE_PRIVATE).edit()
        for (id in ids) p.remove("svc_$id")
        p.apply()
    }

    companion object {
        // App 离开前台时刷新所有实例。
        fun refreshAll(context: Context) {
            val mgr = AppWidgetManager.getInstance(context)
            val cn = ComponentName(context, UsageWidgetProvider::class.java)
            for (id in mgr.getAppWidgetIds(cn)) renderOne(context, mgr, id)
        }

        private fun barColor(pct: Int): Int = when {
            pct >= 90 -> Color.parseColor("#F85149")
            pct >= 75 -> Color.parseColor("#D29922")
            else -> Color.parseColor("#3FB950")
        }

        private val WIN_ROW = intArrayOf(R.id.w_win0, R.id.w_win1, R.id.w_win2)
        private val WIN_LABEL = intArrayOf(R.id.w_win0_label, R.id.w_win1_label, R.id.w_win2_label)
        private val WIN_RESET = intArrayOf(R.id.w_win0_reset, R.id.w_win1_reset, R.id.w_win2_reset)
        private val WIN_PCT = intArrayOf(R.id.w_win0_pct, R.id.w_win1_pct, R.id.w_win2_pct)
        private val WIN_BAR = intArrayOf(R.id.w_win0_bar, R.id.w_win1_bar, R.id.w_win2_bar)

        // 重置倒计时:>=1 小时显示 "1h25m",否则 "25m";已过/无则空。
        private fun countdown(resetMs: Long): String {
            if (resetMs <= 0) return ""
            val diff = resetMs - System.currentTimeMillis()
            if (diff <= 0) return ""
            val totalMin = (diff / 60000).toInt()
            val h = totalMin / 60
            val m = totalMin % 60
            return if (h >= 1) "${h}h${m}m" else "${m}m"
        }

        fun renderOne(context: Context, mgr: AppWidgetManager, id: Int) {
            val views = RemoteViews(context.packageName, R.layout.usage_widget)
            val flags = PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT

            val serviceId = context.getSharedPreferences("usage_widget", Context.MODE_PRIVATE)
                .getString("svc_$id", null)
            val svc = findService(context, serviceId)

            // 点击行为:未选渠道 → 打开配置页选渠道(部分桌面 pin 后不自动弹配置);已配置 → 打开 App。
            if (serviceId == null) {
                val cfg = Intent(context, WidgetConfigActivity::class.java)
                    .putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, id)
                    .setAction("cfg_$id")
                views.setOnClickPendingIntent(R.id.w_root, PendingIntent.getActivity(context, id, cfg, flags))
            } else {
                val launch = context.packageManager.getLaunchIntentForPackage(context.packageName)
                if (launch != null) {
                    views.setOnClickPendingIntent(R.id.w_root, PendingIntent.getActivity(context, id, launch, flags))
                }
            }

            if (svc == null) {
                showStatus(views, if (serviceId == null) "点击选择渠道" else "打开 App 加载数据", "#8E8E93")
                views.setTextViewText(R.id.w_title, "用量")
                views.setTextColor(R.id.w_dot, Color.GRAY)
                views.setTextViewText(R.id.w_updated, "")
                mgr.updateAppWidget(id, views)
                return
            }

            views.setTextViewText(R.id.w_title, svc.optString("title", "用量"))
            views.setTextColor(R.id.w_dot, try { Color.parseColor(svc.optString("accent", "#8E8E93")) } catch (e: Exception) { Color.GRAY })
            views.setTextViewText(R.id.w_updated, updatedText(context))

            when (svc.optString("kind")) {
                "loading" -> showStatus(views, "加载中…", "#8E8E93")
                "error" -> showStatus(views, svc.optString("message", "出错了"), "#F85149")
                "ok" -> {
                    val balance = svc.optJSONObject("balance")
                    if (balance != null) renderBalance(views, balance)
                    else renderWindows(views, svc.optJSONArray("windows"))
                }
                else -> showStatus(views, "无数据", "#8E8E93")
            }
            mgr.updateAppWidget(id, views)
        }

        private fun findService(context: Context, serviceId: String?): JSONObject? {
            if (serviceId == null) return null
            return try {
                val f = File(context.dataDir, "widget.json")
                if (!f.exists()) return null
                val arr = JSONObject(f.readText()).optJSONArray("services") ?: return null
                (0 until arr.length()).asSequence()
                    .mapNotNull { arr.optJSONObject(it) }
                    .firstOrNull { it.optString("id") == serviceId }
            } catch (e: Exception) {
                null
            }
        }

        private fun updatedText(context: Context): String {
            return try {
                val f = File(context.dataDir, "widget.json")
                val ms = JSONObject(f.readText()).optLong("updatedAtMs", 0L)
                if (ms > 0) "更新于 " + SimpleDateFormat("HH:mm", Locale.getDefault()).format(Date(ms)) else ""
            } catch (e: Exception) { "" }
        }

        private fun showStatus(views: RemoteViews, text: String, color: String) {
            views.setViewVisibility(R.id.w_windows, View.GONE)
            views.setViewVisibility(R.id.w_balance, View.GONE)
            views.setViewVisibility(R.id.w_status, View.VISIBLE)
            views.setTextViewText(R.id.w_status, text)
            views.setTextColor(R.id.w_status, try { Color.parseColor(color) } catch (e: Exception) { Color.GRAY })
        }

        private fun renderWindows(views: RemoteViews, wins: org.json.JSONArray?) {
            views.setViewVisibility(R.id.w_status, View.GONE)
            views.setViewVisibility(R.id.w_balance, View.GONE)
            views.setViewVisibility(R.id.w_windows, View.VISIBLE)
            val n = wins?.length() ?: 0
            if (n == 0) { showStatus(views, "无可展示窗口", "#8E8E93"); return }
            for (i in 0 until 3) {
                if (i < n) {
                    val w = wins!!.getJSONObject(i)
                    val pct = w.optDouble("pct", 0.0).toInt().coerceIn(0, 100)
                    views.setViewVisibility(WIN_ROW[i], View.VISIBLE)
                    views.setTextViewText(WIN_LABEL[i], w.optString("label"))
                    views.setTextViewText(WIN_RESET[i], countdown(w.optLong("resetAtMs", 0L)))
                    views.setTextViewText(WIN_PCT[i], "$pct%")
                    views.setTextColor(WIN_PCT[i], barColor(pct))
                    views.setProgressBar(WIN_BAR[i], 100, pct, false)
                    if (Build.VERSION.SDK_INT >= 31) {
                        views.setColorStateList(WIN_BAR[i], "setProgressTintList", ColorStateList.valueOf(barColor(pct)))
                    }
                } else {
                    views.setViewVisibility(WIN_ROW[i], View.GONE)
                }
            }
        }

        private fun renderBalance(views: RemoteViews, b: JSONObject) {
            views.setViewVisibility(R.id.w_status, View.GONE)
            views.setViewVisibility(R.id.w_windows, View.GONE)
            views.setViewVisibility(R.id.w_balance, View.VISIBLE)
            val cur = b.optString("currency", "$")
            val bal = b.optDouble("balance", 0.0)
            val used = b.optDouble("used", 0.0)
            views.setTextViewText(R.id.w_bal_value, "余额 $cur%.2f".format(bal))
            val req = if (b.isNull("requestCount")) null else b.optLong("requestCount")
            val sub = if (req != null) "消耗 $cur%.2f · %,d 次".format(used, req)
                      else "消耗 $cur%.2f".format(used)
            views.setTextViewText(R.id.w_bal_sub, sub)
        }
    }
}
