package app.usagedashboard

import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.util.TypedValue
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import org.json.JSONObject
import java.io.File

// 添加小组件时弹出的配置页:列出 widget.json 里的渠道,选一个绑定到本 appWidgetId。
class WidgetConfigActivity : Activity() {
    private var appWidgetId = AppWidgetManager.INVALID_APPWIDGET_ID

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // 默认取消:用户直接返回则不添加。
        setResult(RESULT_CANCELED)

        appWidgetId = intent?.extras?.getInt(
            AppWidgetManager.EXTRA_APPWIDGET_ID,
            AppWidgetManager.INVALID_APPWIDGET_ID
        ) ?: AppWidgetManager.INVALID_APPWIDGET_ID
        if (appWidgetId == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish(); return
        }

        setContentView(R.layout.activity_widget_config)
        val list = findViewById<LinearLayout>(R.id.config_list)
        val hint = findViewById<TextView>(R.id.config_hint)

        val services = readServices()
        if (services.isEmpty()) {
            hint.text = "未读到渠道数据,请先打开 App 加载一次再添加小组件。"
            return
        }
        for (svc in services) {
            list.addView(makeButton(svc.first, svc.second))
        }
    }

    // 返回 (id, title) 列表。
    private fun readServices(): List<Pair<String, String>> {
        return try {
            val f = File(dataDir, "widget.json")
            if (!f.exists()) return emptyList()
            val arr = JSONObject(f.readText()).optJSONArray("services") ?: return emptyList()
            (0 until arr.length()).mapNotNull { i ->
                val o = arr.optJSONObject(i) ?: return@mapNotNull null
                val id = o.optString("id"); if (id.isEmpty()) return@mapNotNull null
                id to o.optString("title", id)
            }
        } catch (e: Exception) {
            emptyList()
        }
    }

    private fun dp(v: Int): Int =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, v.toFloat(), resources.displayMetrics).toInt()

    private fun makeButton(id: String, title: String): Button {
        val b = Button(this)
        b.text = title
        b.isAllCaps = false
        b.setTextColor(Color.WHITE)
        b.textSize = 15f
        b.setBackgroundColor(Color.parseColor("#1C1C1E"))
        val lp = LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT)
        lp.bottomMargin = dp(10)
        b.layoutParams = lp
        b.setPadding(dp(16), dp(14), dp(16), dp(14))
        b.setOnClickListener { pick(id) }
        return b
    }

    private fun pick(serviceId: String) {
        getSharedPreferences("usage_widget", MODE_PRIVATE)
            .edit().putString("svc_$appWidgetId", serviceId).apply()
        // 立刻渲染一次。
        val mgr = AppWidgetManager.getInstance(this)
        UsageWidgetProvider.renderOne(this, mgr, appWidgetId)
        // 回传 OK 完成添加。
        val result = Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, appWidgetId)
        setResult(RESULT_OK, result)
        finish()
    }
}
