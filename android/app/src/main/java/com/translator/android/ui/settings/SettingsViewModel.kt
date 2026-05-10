package com.translator.android.ui.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.translator.android.TranslatorApp
import com.translator.android.data.model.Language
import com.translator.android.domain.TranslateUseCase
import com.translator.android.service.OverlayService
import android.content.Intent
import android.os.Build
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

/** Хелпер для вложенного combine (4 значения → 1) */
private data class Quad<A, B, C, D>(val first: A, val second: B, val third: C, val fourth: D)

data class SettingsUiState(
    val apiKey: String = "",
    val apiUrl: String = "https://openrouter.ai/api/v1",
    val modelVersion: String = "openai/gpt-4o",
    val primaryLanguage: Language = Language.ENGLISH,
    val secondaryLanguage: Language = Language.FRENCH,
    val allTargetLanguages: List<Language> = Language.defaults,
    val showBubble: Boolean = false,
    // Саппорт для тестового перевода
    val testSourceText: String = "",
    val testResult: String = "",
    val isTranslating: Boolean = false,
    val error: String? = null,
)

class SettingsViewModel : ViewModel() {
    private val settings = TranslatorApp.instance.settings
    private val translateUseCase = TranslateUseCase()

    private val _uiState = MutableStateFlow(SettingsUiState())
    val uiState: StateFlow<SettingsUiState> = _uiState.asStateFlow()

    init {
        // Подписываемся на Flow из DataStore
        // Используем вложенный combine т.к. больше 5 потоков
        viewModelScope.launch {
            combine(
                combine(
                    settings.apiKeyFlow,
                    settings.apiUrlFlow,
                    settings.modelVersionFlow,
                    settings.primaryLanguageFlow,
                ) { key, url, model, primary ->
                    Quad(key, url, model, primary)
                },
                settings.secondaryLanguageFlow,
                settings.allTargetLanguagesFlow,
                settings.showBubbleFlow,
            ) { quad, secondary, all, bubble ->
                _uiState.value.copy(
                    apiKey = quad.first,
                    apiUrl = quad.second,
                    modelVersion = quad.third,
                    primaryLanguage = quad.fourth,
                    secondaryLanguage = secondary,
                    allTargetLanguages = all,
                    showBubble = bubble,
                )
            }.collect { _uiState.value = it }
        }
    }

    // ---- Сеттеры (пишут в DataStore) ----

    fun updateApiKey(key: String) {
        _uiState.update { it.copy(apiKey = key, error = null) }
        viewModelScope.launch { settings.setApiKey(key) }
    }

    fun updateApiUrl(url: String) {
        _uiState.update { it.copy(apiUrl = url, error = null) }
        viewModelScope.launch { settings.setApiUrl(url) }
    }

    fun updateModelVersion(model: String) {
        _uiState.update { it.copy(modelVersion = model, error = null) }
        viewModelScope.launch { settings.setModelVersion(model) }
    }

    fun updatePrimaryLanguage(lang: Language) {
        _uiState.update { it.copy(primaryLanguage = lang, error = null) }
        viewModelScope.launch { settings.setPrimaryLanguage(lang) }
    }

    fun updateSecondaryLanguage(lang: Language) {
        _uiState.update { it.copy(secondaryLanguage = lang, error = null) }
        viewModelScope.launch { settings.setSecondaryLanguage(lang) }
    }

    fun updateAllTargetLanguages(langs: List<Language>) {
        _uiState.update { it.copy(allTargetLanguages = langs, error = null) }
        viewModelScope.launch { settings.setAllTargetLanguages(langs) }
    }

    fun updateShowBubble(show: Boolean) {
        _uiState.update { it.copy(showBubble = show, error = null) }
        viewModelScope.launch {
            settings.setShowBubble(show)
            val context = TranslatorApp.instance
            val intent = Intent(context, OverlayService::class.java)
            if (show) {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    context.startForegroundService(intent)
                } else {
                    context.startService(intent)
                }
            } else {
                context.stopService(intent)
            }
        }
    }

    // ---- Тестовый перевод ----

    fun updateTestSourceText(text: String) {
        _uiState.update { it.copy(testSourceText = text, error = null) }
    }

    fun performTestTranslation() {
        val state = _uiState.value
        if (state.testSourceText.isBlank()) return

        viewModelScope.launch {
            _uiState.update { it.copy(isTranslating = true, error = null, testResult = "") }

            val result = translateUseCase.translate(
                text = state.testSourceText,
                targetLang = null,  // Авто-выбор
                apiKey = state.apiKey,
                apiUrl = state.apiUrl,
                model = state.modelVersion,
                primaryLang = state.primaryLanguage,
                secondaryLang = state.secondaryLanguage,
                lastLang = com.translator.android.data.SettingsDataStore.DEFAULT_LAST_LANG,
            )

            result.fold(
                onSuccess = { (translated, lang) ->
                    _uiState.update { it.copy(testResult = translated, isTranslating = false) }
                },
                onFailure = { e ->
                    _uiState.update { it.copy(error = e.message, isTranslating = false) }
                },
            )
        }
    }

    fun clearError() {
        _uiState.update { it.copy(error = null) }
    }
}
