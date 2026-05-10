package com.translator.android.ui.settings

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import com.translator.android.data.model.Language

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    uiState: SettingsUiState,
    onUpdateApiKey: (String) -> Unit,
    onUpdateApiUrl: (String) -> Unit,
    onUpdateModel: (String) -> Unit,
    onUpdatePrimary: (Language) -> Unit,
    onUpdateSecondary: (Language) -> Unit,
    onUpdateAllTarget: (List<Language>) -> Unit,
    onUpdateShowBubble: (Boolean) -> Unit,
    onUpdateTestText: (String) -> Unit,
    onTranslate: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Translator") },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer,
                )
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(24.dp),
        ) {
            // ============================================
            // API Section
            // ============================================
            SectionHeader("API")

            OutlinedTextField(
                value = uiState.apiKey,
                onValueChange = onUpdateApiKey,
                label = { Text("API Key") },
                placeholder = { Text("sk-or-...") },
                visualTransformation = PasswordVisualTransformation(),
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            OutlinedTextField(
                value = uiState.apiUrl,
                onValueChange = onUpdateApiUrl,
                label = { Text("API URL") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            OutlinedTextField(
                value = uiState.modelVersion,
                onValueChange = onUpdateModel,
                label = { Text("Model") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )

            // ============================================
            // Languages Section
            // ============================================
            SectionHeader("Языки")

            LanguageDropdown(
                label = "Primary",
                selected = uiState.primaryLanguage,
                languages = Language.entries.toList(),
                onSelect = onUpdatePrimary,
            )

            LanguageDropdown(
                label = "Secondary",
                selected = uiState.secondaryLanguage,
                languages = Language.entries.toList(),
                onSelect = onUpdateSecondary,
            )

            // Мультивыбор allTargetLanguages
            Text(
                text = "Доступные языки",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            AllTargetLanguagesChips(
                selected = uiState.allTargetLanguages,
                allLanguages = Language.entries.toList(),
                onUpdate = onUpdateAllTarget,
            )

            // ============================================
            // Bubble Section
            // ============================================
            SectionHeader("Бабл")

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Показывать плавающий бабл")
                Switch(
                    checked = uiState.showBubble,
                    onCheckedChange = onUpdateShowBubble,
                )
            }

            // ============================================
            // Test Translation (preview for Phase 2)
            // ============================================
            SectionHeader("Тестовый перевод")

            OutlinedTextField(
                value = uiState.testSourceText,
                onValueChange = onUpdateTestText,
                label = { Text("Текст для перевода") },
                minLines = 2,
                maxLines = 4,
                modifier = Modifier.fillMaxWidth(),
            )

            Button(
                onClick = onTranslate,
                enabled = uiState.testSourceText.isNotBlank() && !uiState.isTranslating,
                modifier = Modifier.align(Alignment.End),
            ) {
                if (uiState.isTranslating) {
                    CircularProgressIndicator(
                        modifier = Modifier.size(18.dp),
                        strokeWidth = 2.dp,
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                }
                Text("Перевести")
            }

            // Результат
            if (uiState.testResult.isNotBlank()) {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.secondaryContainer,
                    ),
                ) {
                    Text(
                        text = uiState.testResult,
                        modifier = Modifier.padding(12.dp),
                        style = MaterialTheme.typography.bodyLarge,
                    )
                }
            }

            // Ошибка
            uiState.error?.let { err ->
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.errorContainer,
                    ),
                ) {
                    Text(
                        text = err,
                        modifier = Modifier
                            .fillMaxWidth()
                            .verticalScroll(rememberScrollState())
                            .padding(12.dp),
                        color = MaterialTheme.colorScheme.onErrorContainer,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace,
                    )
                }
            }
        }
    }
}

@Composable
private fun SectionHeader(title: String) {
    Text(
        text = title,
        style = MaterialTheme.typography.titleMedium,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(top = 8.dp),
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun LanguageDropdown(
    label: String,
    selected: Language,
    languages: List<Language>,
    onSelect: (Language) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = it },
    ) {
        OutlinedTextField(
            value = selected.displayName,
            onValueChange = {},
            readOnly = true,
            label = { Text(label) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
            modifier = Modifier.menuAnchor().fillMaxWidth(),
        )
        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
        ) {
            languages.forEach { lang ->
                DropdownMenuItem(
                    text = { Text(lang.displayName) },
                    onClick = {
                        onSelect(lang)
                        expanded = false
                    },
                )
            }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun AllTargetLanguagesChips(
    selected: List<Language>,
    allLanguages: List<Language>,
    onUpdate: (List<Language>) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        // Показываем чипсами
        androidx.compose.foundation.layout.FlowRow(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            allLanguages.forEach { lang ->
                FilterChip(
                    selected = lang in selected,
                    onClick = {
                        val new = if (lang in selected) selected - lang else selected + lang
                        if (new.isNotEmpty()) onUpdate(new)
                    },
                    label = { Text(lang.isoCode) },
                )
            }
        }
    }
}
