package app.renovador.bg

import android.app.Activity
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class ScheduleNextArgs {
    lateinit var at: String
}

@TauriPlugin
class RenovadorBgPlugin(private val activity: Activity) : Plugin(activity) {
    override fun load(webView: WebView) {
        super.load(webView)
        RenewScheduler.ensurePeriodic(activity.applicationContext)
    }

    @Command
    fun scheduleNext(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(ScheduleNextArgs::class.java)
            RenewScheduler.scheduleNext(activity.applicationContext, args.at)
            invoke.resolve()
        } catch (e: Exception) {
            invoke.reject(e.message ?: "falha ao agendar")
        }
    }

    @Command
    fun getDbKey(invoke: Invoke) {
        try {
            val key = KeyVault(activity.applicationContext).getOrCreateDbKey()
            val ret = JSObject()
            try {
                ret.put("keyHex", KeyWrapFormat.toHex(key))
            } finally {
                key.fill(0)
            }
            invoke.resolve(ret)
        } catch (e: Exception) {
            invoke.reject(e.message ?: "falha ao ler a chave")
        }
    }
}
