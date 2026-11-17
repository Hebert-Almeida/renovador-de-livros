package app.renovador.bg

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import org.json.JSONObject

/** Texto pronto de uma notificação (gerado pelo núcleo, igual ao do desktop). */
data class Note(val title: String, val body: String)

/**
 * Mostra as notificações de um ciclo em segundo plano. O núcleo já devolve o
 * texto em `notes` (vazio se o usuário desligou as notificações nos Ajustes).
 */
object BackgroundNotifier {
    private const val CHANNEL_ID = "renovacoes"
    private const val TAG = "BackgroundNotifier"

    /** Lê `notes` do JSON de `run_background_cycle`; JSON inválido ou com erro não gera nota. */
    fun parseNotes(reportJson: String): List<Note> = try {
        val arr = JSONObject(reportJson).optJSONArray("notes") ?: return emptyList()
        (0 until arr.length()).mapNotNull { i ->
            val o = arr.optJSONObject(i) ?: return@mapNotNull null
            val title = o.optString("title")
            val body = o.optString("body")
            if (title.isEmpty()) null else Note(title, body)
        }
    } catch (e: Exception) {
        Log.w(TAG, "relatório sem notas legíveis", e)
        emptyList()
    }

    fun show(context: Context, notes: List<Note>) {
        if (notes.isEmpty()) return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            Log.i(TAG, "sem permissão de notificação; ${notes.size} nota(s) descartada(s)")
            return
        }
        ensureChannel(context)
        val manager = NotificationManagerCompat.from(context)
        val icon = context.applicationInfo.icon.takeIf { it != 0 } ?: android.R.drawable.ic_dialog_info
        val base = (System.currentTimeMillis() % Int.MAX_VALUE).toInt()
        notes.forEachIndexed { i, note ->
            val n = NotificationCompat.Builder(context, CHANNEL_ID)
                .setSmallIcon(icon)
                .setContentTitle(note.title)
                .setContentText(note.body)
                .setStyle(NotificationCompat.BigTextStyle().bigText(note.body))
                .setAutoCancel(true)
                .build()
            try {
                manager.notify(base + i, n)
            } catch (e: SecurityException) {
                Log.w(TAG, "notificação recusada pelo sistema", e)
            }
        }
    }

    private fun ensureChannel(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val nm = context.getSystemService(NotificationManager::class.java) ?: return
        if (nm.getNotificationChannel(CHANNEL_ID) != null) return
        nm.createNotificationChannel(
            NotificationChannel(CHANNEL_ID, "Renovações", NotificationManager.IMPORTANCE_DEFAULT).apply {
                description = "Avisos de livros renovados, falhas e vencimentos"
            },
        )
    }
}
