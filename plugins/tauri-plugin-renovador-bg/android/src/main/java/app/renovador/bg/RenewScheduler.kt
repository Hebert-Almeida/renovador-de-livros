package app.renovador.bg

import android.content.Context
import androidx.work.BackoffPolicy
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import java.time.Instant
import java.util.concurrent.TimeUnit

/** Agenda o [RenewWorker] no WorkManager seguindo a [SchedulePolicy]. */
object RenewScheduler {
    private fun constraints(): Constraints =
        Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()

    /** Garante o periódico de 6 h (mantém o existente para não reiniciar a contagem). */
    fun ensurePeriodic(context: Context) {
        val request = PeriodicWorkRequestBuilder<RenewWorker>(
            SchedulePolicy.PERIODIC_INTERVAL.toMillis(), TimeUnit.MILLISECONDS,
            SchedulePolicy.PERIODIC_FLEX.toMillis(), TimeUnit.MILLISECONDS,
        )
            .setConstraints(constraints())
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, SchedulePolicy.BACKOFF.toMillis(), TimeUnit.MILLISECONDS)
            .build()
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            SchedulePolicy.PERIODIC_WORK_NAME, ExistingPeriodicWorkPolicy.KEEP, request,
        )
    }

    /** Agenda (substituindo o anterior) a execução única no horário planejado. */
    fun scheduleNext(context: Context, at: String, now: Instant = Instant.now()): OneTimeRequest {
        val plan = SchedulePolicy.oneTime(at, now)
        val request = OneTimeWorkRequestBuilder<RenewWorker>()
            .setInitialDelay(plan.initialDelay.toMillis(), TimeUnit.MILLISECONDS)
            .setConstraints(constraints())
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, SchedulePolicy.BACKOFF.toMillis(), TimeUnit.MILLISECONDS)
            .build()
        WorkManager.getInstance(context).enqueueUniqueWork(
            SchedulePolicy.ONE_TIME_WORK_NAME, ExistingWorkPolicy.REPLACE, request,
        )
        return plan
    }
}
