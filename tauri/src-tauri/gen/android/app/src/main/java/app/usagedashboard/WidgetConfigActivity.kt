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
import android.widget.RadioButton
import android.widget.RadioGroup
import android.widget.TextView
import org.json.JSONObject
import java.io.File

// 添加小组件时弹出的配置页:列出 widget.json 里的渠道,选一个绑定到本 appWidgetId。
class WidgetConfigActivity : Activity() {
    private var appWidgetId = AppWidgetManager.INVALID_APPWIDGET_ID
    private lateinit var styleGroup: RadioGroup

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
        styleGroup = findViewById(R.id.config_style)
        styleGroup.check(
            when (getSharedPreferences("usage_widget", MODE_PRIVATE).getString("style_$appWidgetId", "large")) {
                "compact" -> R.id.style_compact
                "standard" -> R.id.style_standard
                else -> R.id.style_large
            }
        )

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
                id to displayTitle(id, o.optString("title", id))
            }
        } catch (e: Exception) {
            emptyList()
        }
    }

    private fun dp(v: Int): Int =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, v.toFloat(), resources.displayMetrics).toInt()

    private fun displayTitle(id: String, title: String): String =
        if (id == "codex") "Codex" else title

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
        val style = when (styleGroup.checkedRadioButtonId) {
            R.id.style_compact -> "compact"
            R.id.style_standard -> "standard"
            else -> "large"
        }
        getSharedPreferences("usage_widget", MODE_PRIVATE)
            .edit()
            .putString("svc_$appWidgetId", serviceId)
            .putString("style_$appWidgetId", style)
            .apply()
        // 立刻渲染一次。
        val mgr = AppWidgetManager.getInstance(this)
        UsageWidgetProvider.renderOne(this, mgr, appWidgetId)
        // 回传 OK 完成添加。
        val result = Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, appWidgetId)
        setResult(RESULT_OK, result)
        finish()
    }
}

// 2x2 双服务小组件配置页:分别选择上方和下方渠道。
class DoubleWidgetConfigActivity : Activity() {
    private var appWidgetId = AppWidgetManager.INVALID_APPWIDGET_ID
    private lateinit var styleGroup: RadioGroup
    private lateinit var topGroup: RadioGroup
    private lateinit var bottomGroup: RadioGroup
    private val topChoices = mutableMapOf<Int, String>()
    private val bottomChoices = mutableMapOf<Int, String>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setResult(RESULT_CANCELED)

        appWidgetId = intent?.extras?.getInt(
            AppWidgetManager.EXTRA_APPWIDGET_ID,
            AppWidgetManager.INVALID_APPWIDGET_ID
        ) ?: AppWidgetManager.INVALID_APPWIDGET_ID
        if (appWidgetId == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish(); return
        }

        setContentView(R.layout.activity_double_widget_config)
        val hint = findViewById<TextView>(R.id.double_config_hint)
        styleGroup = findViewById(R.id.double_config_style)
        topGroup = findViewById(R.id.double_top_list)
        bottomGroup = findViewById(R.id.double_bottom_list)

        styleGroup.check(
            when (getSharedPreferences("usage_widget", MODE_PRIVATE).getString("double_style_$appWidgetId", "large")) {
                "compact" -> R.id.double_style_compact
                "standard" -> R.id.double_style_standard
                else -> R.id.double_style_large
            }
        )

        val services = readServices()
        if (services.isEmpty()) {
            hint.text = "未读到渠道数据,请先打开 App 加载一次再添加小组件。"
            findViewById<Button>(R.id.double_config_confirm).isEnabled = false
            return
        }

        for (svc in services) {
            topGroup.addView(makeRadio(svc.first, svc.second, topChoices))
            bottomGroup.addView(makeRadio(svc.first, svc.second, bottomChoices))
        }

        val prefs = getSharedPreferences("usage_widget", MODE_PRIVATE)
        val savedTop = prefs.getString("double_top_$appWidgetId", null)
        val savedBottom = prefs.getString("double_bottom_$appWidgetId", null)
        topGroup.check(choiceId(topChoices, savedTop ?: services.first().first))
        bottomGroup.check(choiceId(bottomChoices, savedBottom ?: services.getOrNull(1)?.first ?: services.first().first))

        findViewById<Button>(R.id.double_config_confirm).setOnClickListener { pick() }
    }

    private fun readServices(): List<Pair<String, String>> {
        return try {
            val f = File(dataDir, "widget.json")
            if (!f.exists()) return emptyList()
            val arr = JSONObject(f.readText()).optJSONArray("services") ?: return emptyList()
            (0 until arr.length()).mapNotNull { i ->
                val o = arr.optJSONObject(i) ?: return@mapNotNull null
                val id = o.optString("id"); if (id.isEmpty()) return@mapNotNull null
                id to UsageWidgetProvider.displayTitle(id, o.optString("title", id))
            }
        } catch (e: Exception) {
            emptyList()
        }
    }

    private fun makeRadio(id: String, title: String, choices: MutableMap<Int, String>): RadioButton {
        val b = RadioButton(this)
        b.id = View.generateViewId()
        choices[b.id] = id
        b.text = title
        b.setTextColor(Color.WHITE)
        b.textSize = 15f
        b.setBackgroundColor(Color.parseColor("#1C1C1E"))
        val lp = RadioGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT)
        lp.bottomMargin = dp(10)
        b.layoutParams = lp
        b.setPadding(dp(14), dp(12), dp(14), dp(12))
        return b
    }

    private fun dp(v: Int): Int =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, v.toFloat(), resources.displayMetrics).toInt()

    private fun choiceId(choices: Map<Int, String>, serviceId: String): Int {
        return choices.entries.firstOrNull { it.value == serviceId }?.key ?: choices.keys.first()
    }

    private fun pick() {
        val top = topChoices[topGroup.checkedRadioButtonId]
        val bottom = bottomChoices[bottomGroup.checkedRadioButtonId]
        if (top == null || bottom == null) {
            findViewById<TextView>(R.id.double_config_hint).text = "请选择上方和下方渠道"
            return
        }
        val style = when (styleGroup.checkedRadioButtonId) {
            R.id.double_style_compact -> "compact"
            R.id.double_style_standard -> "standard"
            else -> "large"
        }
        getSharedPreferences("usage_widget", MODE_PRIVATE)
            .edit()
            .putString("double_top_$appWidgetId", top)
            .putString("double_bottom_$appWidgetId", bottom)
            .putString("double_style_$appWidgetId", style)
            .apply()

        val mgr = AppWidgetManager.getInstance(this)
        UsageDoubleWidgetProvider.renderOne(this, mgr, appWidgetId)
        val result = Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, appWidgetId)
        setResult(RESULT_OK, result)
        finish()
    }
}
