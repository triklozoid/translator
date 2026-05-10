package com.translator.android.language

import com.translator.android.data.model.Language

/**
 * Порт choose_target_language() из Rust-версии (ui.rs).
 *
 * Алгоритм из README.md:
 * 1. Если источник ≠ primary → перевести на primary
 * 2. Если источник = primary и lastLang ≠ primary → lastLang
 * 3. Иначе → secondary
 */
object LanguageSelector {

    fun chooseTargetLanguage(
        sourceLang: Language?,
        primaryLang: Language,
        secondaryLang: Language,
        lastLang: Language,
    ): Language {
        // 1. Если источник не primary → primary
        if (sourceLang == null || sourceLang != primaryLang) {
            return primaryLang
        }

        // 2. Источник = primary, lastLang осмыслен → lastLang
        if (lastLang != primaryLang) {
            return lastLang
        }

        // 3. Fallback → secondary
        return secondaryLang
    }
}
