package app.renovador.bg

/** Ponte JNI para `renovador_core::ffi::run_background_cycle` (lado Rust em `src/jni_bridge.rs`). */
object NativeBridge {
    /** Nome da biblioteca do app (`[lib] name` do `src-tauri`). */
    private const val LIBRARY = "renovador_lib"

    @Volatile
    private var loaded = false

    /** Carrega a biblioteca nativa; o processo pode ter sido iniciado só pelo WorkManager. */
    @Synchronized
    fun ensureLoaded() {
        if (!loaded) {
            System.loadLibrary(LIBRARY)
            loaded = true
        }
    }

    /** Devolve o `CycleReport` em JSON ou `{"error": "..."}`. */
    external fun runBackgroundCycle(dbPath: String, key: ByteArray): String
}
