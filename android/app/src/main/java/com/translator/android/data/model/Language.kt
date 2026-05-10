package com.translator.android.data.model

/**
 * Языки перевода. ISO 639-1 коды.
 *
 * Порт lingua::Language из Rust-версии.
 * Ограниченный набор для v1, расширяемый.
 */
enum class Language(val isoCode: String, val displayName: String) {
    ENGLISH("EN", "English"),
    RUSSIAN("RU", "Русский"),
    FRENCH("FR", "Français"),
    ITALIAN("IT", "Italiano"),
    POLISH("PL", "Polski"),
    GERMAN("DE", "Deutsch"),
    SPANISH("ES", "Español"),
    PORTUGUESE("PT", "Português"),
    CHINESE("ZH", "中文"),
    JAPANESE("JA", "日本語"),
    KOREAN("KO", "한국어"),
    ARABIC("AR", "العربية"),
    TURKISH("TR", "Türkçe"),
    UKRAINIAN("UK", "Українська"),
    DUTCH("NL", "Nederlands");

    companion object {
        /** Найти язык по ISO-коду, null если не найден */
        fun byIsoCode(code: String): Language? =
            entries.find { it.isoCode.equals(code, ignoreCase = true) }

        /** Языки по умолчанию для списка allTargetLanguages */
        val defaults: List<Language> = listOf(ENGLISH, FRENCH, ITALIAN, POLISH)
    }

    override fun toString(): String = displayName
}
