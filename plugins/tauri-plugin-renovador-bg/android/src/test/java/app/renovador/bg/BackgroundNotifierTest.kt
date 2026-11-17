package app.renovador.bg

import org.junit.Assert.assertEquals
import org.junit.Test

class BackgroundNotifierTest {
    @Test
    fun parseNotesReadsTitleAndBody() {
        val json = "{\"notes\":[{\"title\":\"Livro renovado\",\"body\":\"“Dom Casmurro” agora vence em 30/11/2026.\"}],\"outcomes\":[]}"
        assertEquals(
            listOf(Note("Livro renovado", "“Dom Casmurro” agora vence em 30/11/2026.")),
            BackgroundNotifier.parseNotes(json),
        )
    }

    @Test
    fun parseNotesIgnoresErrorsAndGarbage() {
        assertEquals(emptyList<Note>(), BackgroundNotifier.parseNotes("{\"error\":\"falhou\"}"))
        assertEquals(emptyList<Note>(), BackgroundNotifier.parseNotes("não é json"))
        assertEquals(emptyList<Note>(), BackgroundNotifier.parseNotes("{\"notes\":[{\"body\":\"sem título\"}]}"))
    }
}
