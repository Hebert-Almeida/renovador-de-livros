package app.renovador.bg

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Guarda a chave do SQLCipher (32 bytes aleatórios) envolvida por uma chave
 * AES-256-GCM que nunca sai do AndroidKeyStore.
 */
class KeyVault(context: Context) {
    // Fora do backup do Android: a chave do AndroidKeyStore não é restaurada em outro aparelho.
    private val file = File(context.applicationContext.noBackupFilesDir, BLOB_FILE)

    /** Devolve a chave do banco, criando-a na primeira chamada. O chamador deve zerar o array. */
    fun getOrCreateDbKey(): ByteArray = synchronized(LOCK) {
        if (file.exists()) {
            return unwrap(file.readBytes())
        }
        val dbKey = ByteArray(KeyWrapFormat.DB_KEY_LENGTH).also { SecureRandom().nextBytes(it) }
        try {
            val tmp = File(file.parentFile, "$BLOB_FILE.tmp")
            tmp.writeBytes(wrap(dbKey))
            check(tmp.renameTo(file)) { "falha ao gravar a chave envolvida" }
        } catch (e: Exception) {
            dbKey.fill(0)
            throw e
        }
        dbKey
    }

    private fun wrap(plain: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, wrappingKey())
        return KeyWrapFormat.encode(cipher.iv, cipher.doFinal(plain))
    }

    private fun unwrap(blob: ByteArray): ByteArray {
        val (iv, ct) = KeyWrapFormat.decode(blob)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, wrappingKey(), GCMParameterSpec(KeyWrapFormat.TAG_BITS, iv))
        return cipher.doFinal(ct)
    }

    private fun wrappingKey(): SecretKey {
        val ks = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        (ks.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        gen.init(
            KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setRandomizedEncryptionRequired(true)
                .build(),
        )
        return gen.generateKey()
    }

    companion object {
        /** Plugin e worker podem rodar ao mesmo tempo no mesmo processo. */
        private val LOCK = Any()
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val ALIAS = "renovador_db_wrap_v1"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val BLOB_FILE = "db_key_wrapped.bin"
    }
}
