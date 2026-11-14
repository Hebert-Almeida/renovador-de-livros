package app.renovador.bg

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
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
    private val prefs = context.applicationContext
        .getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    /** Devolve a chave do banco, criando-a na primeira chamada. O chamador deve zerar o array. */
    fun getOrCreateDbKey(): ByteArray = synchronized(LOCK) {
        prefs.getString(PREF_WRAPPED, null)?.let { stored ->
            return unwrap(Base64.decode(stored, Base64.NO_WRAP))
        }
        val dbKey = ByteArray(KeyWrapFormat.DB_KEY_LENGTH).also { SecureRandom().nextBytes(it) }
        val blob = wrap(dbKey)
        check(prefs.edit().putString(PREF_WRAPPED, Base64.encodeToString(blob, Base64.NO_WRAP)).commit()) {
            "falha ao gravar a chave envolvida"
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
        private const val PREFS = "renovador_bg"
        private const val PREF_WRAPPED = "db_key_wrapped"
    }
}
