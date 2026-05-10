package com.translator.android.language

import com.translator.android.data.model.Language
import org.junit.Assert.assertEquals
import org.junit.Test

class LanguageSelectorTest {

    // ========================================================================
    // Rule 1: source ≠ primary → translate to primary
    // ========================================================================
    @Test
    fun `source is not primary — returns primary`() {
        val result = LanguageSelector.chooseTargetLanguage(
            sourceLang = Language.RUSSIAN,
            primaryLang = Language.ENGLISH,
            secondaryLang = Language.FRENCH,
            lastLang = Language.ITALIAN,
        )
        assertEquals(Language.ENGLISH, result)
    }

    @Test
    fun `source is null — returns primary`() {
        val result = LanguageSelector.chooseTargetLanguage(
            sourceLang = null,
            primaryLang = Language.ENGLISH,
            secondaryLang = Language.FRENCH,
            lastLang = Language.ITALIAN,
        )
        assertEquals(Language.ENGLISH, result)
    }

    // ========================================================================
    // Rule 2: source = primary, lastLang ≠ primary → use lastLang
    // ========================================================================
    @Test
    fun `source is primary and lastLang differs — returns lastLang`() {
        val result = LanguageSelector.chooseTargetLanguage(
            sourceLang = Language.ENGLISH,
            primaryLang = Language.ENGLISH,
            secondaryLang = Language.FRENCH,
            lastLang = Language.RUSSIAN,
        )
        assertEquals(Language.RUSSIAN, result)
    }

    // ========================================================================
    // Rule 3: source = primary, lastLang = primary → fallback to secondary
    // ========================================================================
    @Test
    fun `source is primary and lastLang equals primary — returns secondary`() {
        val result = LanguageSelector.chooseTargetLanguage(
            sourceLang = Language.ENGLISH,
            primaryLang = Language.ENGLISH,
            secondaryLang = Language.FRENCH,
            lastLang = Language.ENGLISH,
        )
        assertEquals(Language.FRENCH, result)
    }
}
