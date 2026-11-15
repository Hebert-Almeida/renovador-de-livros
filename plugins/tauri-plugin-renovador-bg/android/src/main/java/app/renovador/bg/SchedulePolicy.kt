package app.renovador.bg

import java.time.Duration
import java.time.Instant
import java.time.OffsetDateTime
import java.time.format.DateTimeParseException

/** Pedido de execução única calculado a partir do horário planejado. */
data class OneTimeRequest(val runAt: Instant, val initialDelay: Duration)

/**
 * Regras puras de agendamento (sem dependência do Android) para poderem
 * ser testadas na JVM. O WorkManager só recebe o resultado.
 */
object SchedulePolicy {
    /** Intervalo do trabalho periódico (seção 5.4). */
    val PERIODIC_INTERVAL: Duration = Duration.ofHours(6)

    /** Janela flexível do periódico: roda na última hora de cada intervalo. */
    val PERIODIC_FLEX: Duration = Duration.ofHours(1)

    /** Backoff inicial em caso de `retry`. */
    val BACKOFF: Duration = Duration.ofMinutes(15)

    /** Horizonte máximo aceito para um agendamento único. */
    val MAX_HORIZON: Duration = Duration.ofDays(60)

    const val PERIODIC_WORK_NAME = "renovador-bg-periodic"
    const val ONE_TIME_WORK_NAME = "renovador-bg-once"

    /** Interpreta `at` em RFC 3339 com deslocamento (ex.: `2026-11-15T08:30:00-03:00`). */
    fun parseInstant(at: String): Instant {
        val text = at.trim()
        require(text.isNotEmpty()) { "horário vazio" }
        return try {
            OffsetDateTime.parse(text).toInstant()
        } catch (e: DateTimeParseException) {
            throw IllegalArgumentException("horário inválido: $at", e)
        }
    }

    /**
     * Calcula a execução única: horários no passado viram execução imediata
     * (catch-up); horários além de [MAX_HORIZON] são recusados.
     */
    fun oneTime(at: String, now: Instant): OneTimeRequest {
        val runAt = parseInstant(at)
        val delay = Duration.between(now, runAt)
        require(delay <= MAX_HORIZON) { "horário além de ${MAX_HORIZON.toDays()} dias: $at" }
        return OneTimeRequest(runAt, if (delay.isNegative) Duration.ZERO else delay)
    }

    /** Resultado do worker. */
    enum class Outcome { SUCCESS, RETRY, FAILURE }

    private val ERROR_FIELD = Regex("\"error\"\\s*:")
    private val RETRY_TRUE = Regex("\"retry\"\\s*:\\s*true")

    /**
     * Decide o resultado do worker a partir do JSON do núcleo. Só repete
     * quando o núcleo pede (`"retry": true`, falha de rede) e no máximo
     * [maxAttempts] execuções no total; outro erro encerra sem nova tentativa.
     */
    fun outcome(reportJson: String, runAttemptCount: Int, maxAttempts: Int = 3): Outcome {
        val hasError = ERROR_FIELD.containsMatchIn(reportJson)
        val wantsRetry = RETRY_TRUE.containsMatchIn(reportJson)
        return when {
            wantsRetry && runAttemptCount + 1 < maxAttempts -> Outcome.RETRY
            hasError || wantsRetry -> Outcome.FAILURE
            else -> Outcome.SUCCESS
        }
    }
}
