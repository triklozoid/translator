package com.translator.android.domain

/** Типизированные ошибки перевода (порт humanize_error из Rust) */
sealed class TranslationError(message: String, cause: Throwable? = null) : Exception(message, cause) {
    class EmptyText : TranslationError("Текст для перевода пуст")
    class AuthError(detail: String) : TranslationError("Ошибка авторизации. Проверьте API ключ.\n$detail")
    class RateLimit(detail: String) : TranslationError("Слишком много запросов. Попробуйте позже.\n$detail")
    class NetworkError(detail: String) : TranslationError("Ошибка сети. Проверьте подключение к интернету.\n$detail")
    class ServerError(detail: String) : TranslationError("Ошибка сервера. Попробуйте позже.\n$detail")
    class Timeout(detail: String) : TranslationError("Запрос превысил время ожидания.\n$detail")
    class EmptyResponse : TranslationError("API вернул пустой перевод")
    class Unknown(detail: String) : TranslationError(detail)  // detail уже содержит всё
}
