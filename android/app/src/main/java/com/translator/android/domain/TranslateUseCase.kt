package com.translator.android.domain

import android.util.Log
import com.translator.android.api.ApiClient
import com.translator.android.api.dto.ChatRequest
import com.translator.android.api.dto.Message
import com.translator.android.data.model.Language
import com.translator.android.language.LanguageDetector
import com.translator.android.language.LanguageSelector
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.net.ConnectException
import java.net.SocketTimeoutException
import java.net.UnknownHostException

class TranslateUseCase {

    companion object {
        private const val TAG = "TranslateUseCase"
    }

    suspend fun translate(
        text: String,
        targetLang: Language?,
        apiKey: String,
        apiUrl: String,
        model: String,
        primaryLang: Language,
        secondaryLang: Language,
        lastLang: Language,
    ): Result<Pair<String, Language>> = withContext(Dispatchers.IO) {
        try {
            if (text.isBlank()) return@withContext Result.failure(TranslationError.EmptyText())

            val sourceLang = LanguageDetector.detect(text)
            Log.d(TAG, "Detected source language: ${sourceLang?.isoCode ?: "unknown"}")

            val actualTarget = targetLang
                ?: LanguageSelector.chooseTargetLanguage(sourceLang, primaryLang, secondaryLang, lastLang)
            Log.d(TAG, "Target language: ${actualTarget.isoCode}")

            val targetName = if (actualTarget == Language.PORTUGUESE) "European Portuguese"
            else actualTarget.displayName

            val prompt = "Translate the following text into $targetName. Provide only the translation, nothing else.\n\n$text"

            val api = ApiClient.create(apiUrl)
            val request = ChatRequest(
                model = model,
                messages = listOf(Message(role = "user", content = prompt)),
                maxTokens = 4096,
            )

            val auth = "Bearer $apiKey"
            Log.d(TAG, "Calling API: model=$model, textLen=${text.length}")
            val response = api.chatCompletion(auth = auth, request = request)

            val resultText = response.choices?.firstOrNull()?.message?.content?.trim()
                ?.ifEmpty { response.content?.trim() }
                ?: ""

            if (resultText.isEmpty()) {
                Log.w(TAG, "API returned empty response (choices=${response.choices?.size ?: 0})")
                return@withContext Result.failure(TranslationError.EmptyResponse())
            }

            Log.d(TAG, "Translation success: ${resultText.length} chars")
            Result.success(Pair(resultText, actualTarget))
        } catch (e: Exception) {
            Log.e(TAG, "Translation failed", e)
            Result.failure(buildError(e, apiUrl))
        }
    }

    /** Строит детальную ошибку с URL, кодом и телом ответа */
    private fun buildError(e: Exception, apiUrl: String): TranslationError {
        return when (e) {
            is retrofit2.HttpException -> {
                val code = e.code()
                val statusText = e.message() ?: "HTTP $code"

                // Извлекаем тело ответа
                val responseBody = try {
                    e.response()?.errorBody()?.string()
                } catch (ex: Exception) {
                    "Unable to read error body: ${ex.message}"
                }.orEmpty()

                val detail = buildString {
                    appendLine("━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
                    appendLine("HTTP $code $statusText")
                    appendLine("URL: $apiUrl")
                    if (responseBody.isNotBlank()) {
                        appendLine("Response body:")
                        appendLine(responseBody.take(500))
                    }
                    appendLine("━━━━━━━━━━━━━━━━━━━━━━━━━━━━")
                }

                Log.e("TranslateUseCase", detail)

                when (code) {
                    401 -> TranslationError.AuthError(detail)
                    403 -> TranslationError.AuthError(detail)
                    429 -> TranslationError.RateLimit(detail)
                    in 500..599 -> TranslationError.ServerError(detail)
                    else -> TranslationError.Unknown(detail)
                }
            }
            is SocketTimeoutException -> {
                val msg = e.message ?: "Timeout"
                TranslationError.Timeout("Время ожидания истекло.\nURL: $apiUrl\n$msg")
            }
            is ConnectException,
            is UnknownHostException -> {
                val msg = e.message ?: "No connection"
                TranslationError.NetworkError("Нет подключения.\nURL: $apiUrl\n$msg")
            }
            else -> {
                val msg = e.message ?: "Unknown error"
                TranslationError.Unknown(buildString {
                    appendLine("Неизвестная ошибка:")
                    appendLine("URL: $apiUrl")
                    appendLine("Тип: ${e.javaClass.simpleName}")
                    appendLine("Детали: $msg")
                })
            }
        }
    }
}
