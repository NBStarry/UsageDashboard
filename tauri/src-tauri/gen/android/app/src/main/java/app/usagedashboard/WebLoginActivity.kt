package app.usagedashboard

import android.app.Activity
import android.os.Bundle
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Toast
import org.json.JSONObject
import org.json.JSONTokener
import java.io.File

// New-API 网关网页登录:加载网关页让用户正常登录,登录后自动调
// /api/user/token 生成系统访问令牌 + 从 localStorage 读 userId,
// 写入 New-API 凭证文件(同 save_new_api_credentials 的格式),无需手动复制。
class WebLoginActivity : Activity() {
    private var baseUrl = ""
    private var filePath = ""
    private var done = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        baseUrl = (intent.getStringExtra("baseUrl") ?: "").trimEnd('/')
        filePath = intent.getStringExtra("filePath") ?: ""
        if (baseUrl.isEmpty() || filePath.isEmpty()) { finish(); return }

        val web = WebView(this)
        setContentView(web)
        web.settings.javaScriptEnabled = true
        web.settings.domStorageEnabled = true
        web.webViewClient = object : WebViewClient() {
            override fun onPageFinished(view: WebView, url: String) {
                tryExtractUser(view)
            }
        }
        Toast.makeText(this, "登录后将自动获取令牌", Toast.LENGTH_LONG).show()
        web.loadUrl(baseUrl)
    }

    // evaluateJavascript 返回的是 JSON 引号包裹的字符串,解包成内层字符串。
    private fun unquote(s: String?): String {
        if (s == null) return ""
        return try { JSONTokener(s).nextValue() as? String ?: s } catch (e: Exception) { s }
    }

    private fun tryExtractUser(web: WebView) {
        if (done) return
        val js = """
        (function(){
          try {
            var raw = localStorage.getItem('user'); if(!raw) return JSON.stringify({s:'wait'});
            var p = JSON.parse(raw);
            var obj = (p && p.user) ? (typeof p.user==='string'?JSON.parse(p.user):p.user) : p;
            var uid = obj && obj.id;
            if (uid==null) return JSON.stringify({s:'wait'});
            return JSON.stringify({s:'user', uid:uid});
          } catch(e){ return JSON.stringify({s:'wait'}); }
        })()
        """.trimIndent()
        web.evaluateJavascript(js) { res ->
            try {
                val o = JSONObject(unquote(res))
                if (o.optString("s") == "user") generateToken(web, o.optLong("uid"))
            } catch (e: Exception) { }
        }
    }

    private fun generateToken(web: WebView, uid: Long) {
        if (done) return
        val js = """
        (async function(){
          try {
            var r = await fetch('$baseUrl/api/user/token', {headers:{'New-Api-User':'$uid'}, credentials:'include'});
            var j = await r.json();
            if (j && j.success && j.data) return JSON.stringify({s:'ok', token:j.data});
            return JSON.stringify({s:'wait'});
          } catch(e){ return JSON.stringify({s:'wait'}); }
        })()
        """.trimIndent()
        web.evaluateJavascript(js) { res ->
            try {
                val o = JSONObject(unquote(res))
                if (o.optString("s") == "ok") {
                    val token = o.optString("token")
                    if (token.isNotEmpty()) saveAndFinish(token, uid)
                }
            } catch (e: Exception) { }
        }
    }

    private fun saveAndFinish(token: String, uid: Long) {
        if (done) return
        done = true
        val ok = try {
            val doc = JSONObject()
                .put("baseUrl", baseUrl)
                .put("accessToken", token)
                .put("userId", uid)
                .put("quotaPerUnit", 500000)
                .put("currency", "$")
            File(filePath).writeText(doc.toString())
            true
        } catch (e: Exception) { false }
        runOnUiThread {
            Toast.makeText(this, if (ok) "已获取令牌,返回刷新" else "保存失败", Toast.LENGTH_SHORT).show()
            setResult(if (ok) RESULT_OK else RESULT_CANCELED)
            finish()
        }
    }
}
