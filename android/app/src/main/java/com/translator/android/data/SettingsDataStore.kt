package com.translator.android.data

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.*
import androidx.datastore.preferences.preferencesDataStore
import com.translator.android.data.model.Language
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

/** Extension property — один DataStore на приложение */
private val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "settings")

/**
 * Обёртка над Preferences DataStore.
 * Порт config.rs — load_config() / save_config() + resolve_api_key().
 */
class SettingsDataStore(private val context: Context) {

    // ---- Ключи ----
    private object Keys {
        val API_KEY = stringPreferencesKey("api_key")
        val API_URL = stringPreferencesKey("api_url")
        val MODEL_VERSION = stringPreferencesKey("model_version")
        val PRIMARY_LANG = stringPreferencesKey("primary_language")
        val SECONDARY_LANG = stringPreferencesKey("secondary_language")
        val ALL_TARGET_LANGS = stringPreferencesKey("all_target_languages")
        val LAST_TARGET_LANG = stringPreferencesKey("last_target_language")
        val SHOW_BUBBLE = booleanPreferencesKey("show_bubble")
        val CLIPBOARD_TEXT = stringPreferencesKey("clipboard_text")
    }

    // ---- Значения по умолчанию (как Config::default()) ----
    companion object {
        const val DEFAULT_API_URL = "https://openrouter.ai/api/v1"
        const val DEFAULT_MODEL = "openai/gpt-4o"
        val DEFAULT_PRIMARY: Language = Language.ENGLISH
        val DEFAULT_SECONDARY: Language = Language.FRENCH
        val DEFAULT_ALL_LANGS: List<Language> = Language.defaults
        val DEFAULT_LAST_LANG: Language = Language.ENGLISH
        const val DEFAULT_SHOW_BUBBLE = false
    }

    // ========================================================================
    // Flow'ы для чтения
    // ========================================================================

    val apiKeyFlow: Flow<String> = context.dataStore.data.map { prefs ->
        prefs[Keys.API_KEY] ?: ""
    }

    val apiUrlFlow: Flow<String> = context.dataStore.data.map { prefs ->
        prefs[Keys.API_URL] ?: DEFAULT_API_URL
    }

    val modelVersionFlow: Flow<String> = context.dataStore.data.map { prefs ->
        prefs[Keys.MODEL_VERSION] ?: DEFAULT_MODEL
    }

    val primaryLanguageFlow: Flow<Language> = context.dataStore.data.map { prefs ->
        val code = prefs[Keys.PRIMARY_LANG] ?: DEFAULT_PRIMARY.isoCode
        Language.byIsoCode(code) ?: DEFAULT_PRIMARY
    }

    val secondaryLanguageFlow: Flow<Language> = context.dataStore.data.map { prefs ->
        val code = prefs[Keys.SECONDARY_LANG] ?: DEFAULT_SECONDARY.isoCode
        Language.byIsoCode(code) ?: DEFAULT_SECONDARY
    }

    val allTargetLanguagesFlow: Flow<List<Language>> = context.dataStore.data.map { prefs ->
        val raw = prefs[Keys.ALL_TARGET_LANGS]
        if (raw.isNullOrBlank()) DEFAULT_ALL_LANGS
        else raw.split(",").mapNotNull { Language.byIsoCode(it.trim()) }
            .ifEmpty { DEFAULT_ALL_LANGS }
    }

    val lastTargetLanguageFlow: Flow<Language> = context.dataStore.data.map { prefs ->
        val code = prefs[Keys.LAST_TARGET_LANG] ?: DEFAULT_LAST_LANG.isoCode
        Language.byIsoCode(code) ?: DEFAULT_LAST_LANG
    }

    val showBubbleFlow: Flow<Boolean> = context.dataStore.data.map { prefs ->
        prefs[Keys.SHOW_BUBBLE] ?: DEFAULT_SHOW_BUBBLE
    }

    val clipboardTextFlow: Flow<String?> = context.dataStore.data.map { prefs ->
        prefs[Keys.CLIPBOARD_TEXT]
    }

    // ========================================================================
    // Запись (suspend)
    // ========================================================================

    suspend fun setApiKey(key: String) {
        context.dataStore.edit { it[Keys.API_KEY] = key }
    }

    suspend fun setApiUrl(url: String) {
        context.dataStore.edit { it[Keys.API_URL] = url }
    }

    suspend fun setModelVersion(model: String) {
        context.dataStore.edit { it[Keys.MODEL_VERSION] = model }
    }

    suspend fun setPrimaryLanguage(lang: Language) {
        context.dataStore.edit { it[Keys.PRIMARY_LANG] = lang.isoCode }
    }

    suspend fun setSecondaryLanguage(lang: Language) {
        context.dataStore.edit { it[Keys.SECONDARY_LANG] = lang.isoCode }
    }

    suspend fun setAllTargetLanguages(langs: List<Language>) {
        val raw = langs.joinToString(",") { it.isoCode }
        context.dataStore.edit { it[Keys.ALL_TARGET_LANGS] = raw }
    }

    suspend fun setLastTargetLanguage(lang: Language) {
        context.dataStore.edit { it[Keys.LAST_TARGET_LANG] = lang.isoCode }
    }

    suspend fun setShowBubble(show: Boolean) {
        context.dataStore.edit { it[Keys.SHOW_BUBBLE] = show }
    }

    suspend fun setClipboardText(text: String?) {
        context.dataStore.edit { it[Keys.CLIPBOARD_TEXT] = text ?: "" }
    }

    // ========================================================================
    // Snapshots (разовое чтение)
    // ========================================================================

    /** Читает все настройки одним блоком (для инициализации) */
    suspend fun snapshot(): SettingsSnapshot {
        val prefs = context.dataStore.data
        var result: SettingsSnapshot? = null
        prefs.collect { p ->
            if (result == null) {
                result = SettingsSnapshot(
                    apiKey = p[Keys.API_KEY] ?: "",
                    apiUrl = p[Keys.API_URL] ?: DEFAULT_API_URL,
                    modelVersion = p[Keys.MODEL_VERSION] ?: DEFAULT_MODEL,
                    primaryLanguage = Language.byIsoCode(p[Keys.PRIMARY_LANG] ?: "") ?: DEFAULT_PRIMARY,
                    secondaryLanguage = Language.byIsoCode(p[Keys.SECONDARY_LANG] ?: "") ?: DEFAULT_SECONDARY,
                    allTargetLanguages = p[Keys.ALL_TARGET_LANGS]?.split(",")
                        ?.mapNotNull { Language.byIsoCode(it.trim()) }
                        ?.ifEmpty { DEFAULT_ALL_LANGS } ?: DEFAULT_ALL_LANGS,
                    lastTargetLanguage = Language.byIsoCode(p[Keys.LAST_TARGET_LANG] ?: "") ?: DEFAULT_LAST_LANG,
                    showBubble = p[Keys.SHOW_BUBBLE] ?: DEFAULT_SHOW_BUBBLE,
                )
            }
        }
        return result!!
    }
}

/** Снимок всех настроек за один раз */
data class SettingsSnapshot(
    val apiKey: String,
    val apiUrl: String,
    val modelVersion: String,
    val primaryLanguage: Language,
    val secondaryLanguage: Language,
    val allTargetLanguages: List<Language>,
    val lastTargetLanguage: Language,
    val showBubble: Boolean,
)
