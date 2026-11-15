package app.renovador.bg

import java.time.Duration
import java.time.Instant
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class SchedulePolicyTest {
    private val now = Instant.parse("2026-11-14T12:00:00Z")

    @Test
    fun periodicIsSixHours() {
        assertEquals(Duration.ofHours(6), SchedulePolicy.PERIODIC_INTERVAL)
        assertTrue(SchedulePolicy.PERIODIC_FLEX < SchedulePolicy.PERIODIC_INTERVAL)
    }

    @Test
    fun futureTimeWithOffsetGivesExactDelay() {
        val r = SchedulePolicy.oneTime("2026-11-15T08:30:00-03:00", now)
        assertEquals(Instant.parse("2026-11-15T11:30:00Z"), r.runAt)
        assertEquals(Duration.ofHours(23).plusMinutes(30), r.initialDelay)
    }

    @Test
    fun utcSuffixIsAccepted() {
        assertEquals(Duration.ofHours(1), SchedulePolicy.oneTime("2026-11-14T13:00:00Z", now).initialDelay)
    }

    @Test
    fun pastTimeRunsImmediately() {
        assertEquals(Duration.ZERO, SchedulePolicy.oneTime("2026-11-13T07:00:00-03:00", now).initialDelay)
    }

    @Test
    fun nowRunsImmediately() {
        assertEquals(Duration.ZERO, SchedulePolicy.oneTime("2026-11-14T12:00:00Z", now).initialDelay)
    }

    @Test
    fun beyondHorizonIsRejected() {
        assertThrows(IllegalArgumentException::class.java) {
            SchedulePolicy.oneTime("2027-03-01T08:00:00-03:00", now)
        }
    }

    @Test
    fun invalidEmptyOrWithoutOffsetIsRejected() {
        assertThrows(IllegalArgumentException::class.java) { SchedulePolicy.oneTime("amanha", now) }
        assertThrows(IllegalArgumentException::class.java) { SchedulePolicy.oneTime("  ", now) }
        assertThrows(IllegalArgumentException::class.java) { SchedulePolicy.oneTime("2026-11-15T08:30:00", now) }
    }

    @Test
    fun outcomeSuccessWhenNoError() {
        assertEquals(SchedulePolicy.Outcome.SUCCESS, SchedulePolicy.outcome("{\"renewed\":2}", 0))
    }

    @Test
    fun outcomeFailureOnNonRetryableError() {
        assertEquals(SchedulePolicy.Outcome.FAILURE, SchedulePolicy.outcome("{\"error\": \"reserva\"}", 0))
    }

    @Test
    fun outcomeRetriesNetworkErrorsUpToLimit() {
        val json = "{\"error\":\"rede\",\"retry\": true}"
        assertEquals(SchedulePolicy.Outcome.RETRY, SchedulePolicy.outcome(json, 0))
        assertEquals(SchedulePolicy.Outcome.RETRY, SchedulePolicy.outcome(json, 1))
        assertEquals(SchedulePolicy.Outcome.FAILURE, SchedulePolicy.outcome(json, 2))
    }

    @Test
    fun keyWrapRoundTripAndHex() {
        val iv = ByteArray(12) { it.toByte() }
        val ct = ByteArray(48) { (it * 3).toByte() }
        val (iv2, ct2) = KeyWrapFormat.decode(KeyWrapFormat.encode(iv, ct))
        assertArrayEquals(iv, iv2)
        assertArrayEquals(ct, ct2)
        assertEquals("00ff10", KeyWrapFormat.toHex(byteArrayOf(0, -1, 16)))
        assertThrows(IllegalArgumentException::class.java) { KeyWrapFormat.decode(ByteArray(5)) }
    }
}
