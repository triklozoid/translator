package com.translator.android.language

import android.content.Context
import com.google.mlkit.common.model.DownloadConditions
import com.google.mlkit.nl.languageid.LanguageIdentification
import com.google.mlkit.nl.languageid.IdentifiedLanguage
import com.translator.android.data.model.Language
import kotlinx.coroutines.*
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/**
 * Обёртка над ML Kit Language Identification.
 *
 * Порт lingua-rs детекции из Rust-версии.
 * Использует первые 100 символов и таймаут 2 секунды.
 */
object LanguageDetector {

    private val identifier = LanguageIdentification.getClient()

    /**
     * Определяет язык текста.
     * @param text Текст для анализа (используются первые 100 символов)
     * @return Language или null если не удалось определить
     */
    suspend fun detect(text: String): Language? {
        if (text.isBlank()) return null

        val sample = if (text.length > 100) text.take(100) else text

        return try {
            withTimeout(2_000L) {
                suspendCancellableCoroutine { cont ->
                    identifier.identifyLanguage(sample)
                        .addOnSuccessListener { langCode ->
                            val lang = if (langCode == "und" || langCode.isEmpty()) null
                            else Language.byIsoCode(langCode.take(2).uppercase())
                            cont.resume(lang)
                        }
                        .addOnFailureListener { e ->
                            cont.resumeWithException(e)
                        }
                }
            }
        } catch (e: Exception) {
            null // Fallback: не удалось определить → переведём на primary
        }
    }
}
