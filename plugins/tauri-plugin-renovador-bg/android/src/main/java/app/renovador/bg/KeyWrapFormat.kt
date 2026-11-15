package app.renovador.bg

/**
 * Formato da chave do banco envolvida: `versão(1) | iv(12) | ciphertext+tag`.
 * Puro para ser testado na JVM.
 */
object KeyWrapFormat {
    const val VERSION: Byte = 1
    const val IV_LENGTH = 12
    const val TAG_BITS = 128
    const val DB_KEY_LENGTH = 32

    fun encode(iv: ByteArray, ciphertext: ByteArray): ByteArray {
        require(iv.size == IV_LENGTH) { "IV deve ter $IV_LENGTH bytes" }
        return byteArrayOf(VERSION) + iv + ciphertext
    }

    fun decode(blob: ByteArray): Pair<ByteArray, ByteArray> {
        require(blob.size > 1 + IV_LENGTH + TAG_BITS / 8) { "blob curto demais" }
        require(blob[0] == VERSION) { "versão desconhecida: ${blob[0]}" }
        val iv = blob.copyOfRange(1, 1 + IV_LENGTH)
        val ct = blob.copyOfRange(1 + IV_LENGTH, blob.size)
        return iv to ct
    }

    fun toHex(bytes: ByteArray): String {
        val digits = "0123456789abcdef"
        val sb = StringBuilder(bytes.size * 2)
        for (b in bytes) {
            val v = b.toInt() and 0xff
            sb.append(digits[v ushr 4]).append(digits[v and 0x0f])
        }
        return sb.toString()
    }
}
