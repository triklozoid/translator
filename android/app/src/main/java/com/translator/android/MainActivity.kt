package com.translator.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.runtime.getValue
import com.translator.android.ui.settings.SettingsScreen
import com.translator.android.ui.settings.SettingsViewModel
import com.translator.android.ui.theme.TranslatorTheme

class MainActivity : ComponentActivity() {

    private val viewModel: SettingsViewModel by lazy { SettingsViewModel() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            TranslatorTheme {
                val uiState by viewModel.uiState.collectAsStateWithLifecycle()

                SettingsScreen(
                    uiState = uiState,
                    onUpdateApiKey = viewModel::updateApiKey,
                    onUpdateApiUrl = viewModel::updateApiUrl,
                    onUpdateModel = viewModel::updateModelVersion,
                    onUpdatePrimary = viewModel::updatePrimaryLanguage,
                    onUpdateSecondary = viewModel::updateSecondaryLanguage,
                    onUpdateAllTarget = viewModel::updateAllTargetLanguages,
                    onUpdateShowBubble = viewModel::updateShowBubble,
                    onUpdateTestText = viewModel::updateTestSourceText,
                    onTranslate = viewModel::performTestTranslation,
                )
            }
        }
    }
}
