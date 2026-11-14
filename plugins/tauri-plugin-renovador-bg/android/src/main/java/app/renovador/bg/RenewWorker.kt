package app.renovador.bg

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Executa um ciclo do núcleo em segundo plano (disparado pelo WorkManager). */
class RenewWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        val dbPath = inputData.getString(KEY_DB_PATH) ?: defaultDbPath(applicationContext)
        val report = try {
            NativeBridge.ensureLoaded()
            val key = KeyVault(applicationContext).getOrCreateDbKey()
            try {
                NativeBridge.runBackgroundCycle(dbPath, key)
            } finally {
                key.fill(0)
            }
        } catch (t: Throwable) {
            Log.e(TAG, "ciclo em segundo plano falhou", t)
            return@withContext Result.failure()
        }
        when (SchedulePolicy.outcome(report, runAttemptCount)) {
            SchedulePolicy.Outcome.SUCCESS -> Result.success()
            SchedulePolicy.Outcome.RETRY -> Result.retry()
            SchedulePolicy.Outcome.FAILURE -> {
                Log.w(TAG, "ciclo terminou com erro")
                Result.failure()
            }
        }
    }

    companion object {
        const val KEY_DB_PATH = "dbPath"
        private const val TAG = "RenewWorker"

        /** Arquivo padrão do banco no diretório de dados do app (confirmar na Onda 3). */
        fun defaultDbPath(context: Context): String =
            File(context.applicationInfo.dataDir, "renovador.db").absolutePath
    }
}
