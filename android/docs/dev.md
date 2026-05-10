# Android Development Quickstart

Руководство по быстрому старту разработки Android-приложений для порта Clipboard Translator под Android.

> **💡 CLI-first**: проект настроен так, что Android Studio **не нужен**.
> Всё через `make` и `gradlew`. См. раздел 2.2.

## 1. Обзор

Современный стек Android-разработки (2024–2025):
- **Язык**: Kotlin (официально рекомендован Google)
- **UI-фреймворк**: Jetpack Compose (декларативный UI, замена XML-вёрстке)
- **Сборка**: Gradle с Kotlin DSL (`.gradle.kts`)
- **IDE** (опционально): Android Studio

## 2. Установка инструментов

Два варианта на выбор:

### 2.1 Через Android Studio (GUI)

Скачать с https://developer.android.com/studio. При установке автоматически поставится:
- Android SDK
- Эмулятор (Android Virtual Device / AVD)
- Gradle
- JDK (в комплекте)

### 2.2 CLI-only (без GUI, make + gradlew)

Ничего не надо ставить глобально — весь SDK скачивается локально в `android/sdk/`:

```bash
cd android/
make setup     # Download Android SDK + generate project
make build     # Build debug APK
make install   # Build & install on device via adb
```

Что происходит при `make setup`:
1. Скачиваются Android command-line tools (zip) → `sdk/cmdline-tools/`
2. Через `sdkmanager` ставятся platform, build-tools, platform-tools
3. Генерируется `gradlew` (Gradle wrapper)
4. Готово — можно собирать APK

Требования к хосту:
- `curl`, `unzip`, `java` (JDK 17+)
- `gradle` (только один раз чтобы сгенерировать `gradlew`, потом не нужен)
- `adb` для установки на устройство (ставится через `make sdk` в `sdk/platform-tools/`)

## 3. Структура проекта

Типичная структура Android-проекта с Jetpack Compose:

```
translator-android/
├── build.gradle.kts              # Project-level: общие настройки, плагины
├── settings.gradle.kts           # Список модулей проекта
├── gradle.properties             # Свойства JVM и Gradle
├── gradle/
│   └── libs.versions.toml        # Version catalog: версии зависимостей
│
└── app/                          # Главный модуль приложения
    ├── build.gradle.kts          # App-level: SDK, зависимости, Compose
    ├── proguard-rules.pro        # Правила обфускации для release
    └── src/
        ├── main/
        │   ├── AndroidManifest.xml   # Метаданные приложения (имя, permissions, activity)
        │   ├── java/com/translator/android/
        │   │   ├── MainActivity.kt   # Точка входа приложения
        │   │   └── ui/               # Экранные composable-функции
        │   └── res/                  # Ресурсы (строки, цвета, drawable...)
        └── test/                     # Unit-тесты
```

## 4. Создание нового проекта

1. **Android Studio** → `New Project` → выбрать шаблон `Empty Activity` (с Jetpack Compose)
2. Задать:
   - **Name**: `Translator`
   - **Package name**: `com.translator.android`
   - **Language**: `Kotlin`
   - **Minimum SDK**: `API 24` (Android 7.0) — покрывает ~96% устройств
3. Дождаться завершения Gradle Sync

## 5. Ключевые файлы — объяснение

### 5.1 `app/build.gradle.kts` (модуль приложения)

```kotlin
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")  // Kotlin 2.0+
}

android {
    namespace = "com.translator.android"
    compileSdk = 35          // Версия SDK для компиляции (бери последнюю)

    defaultConfig {
        applicationId = "com.translator.android"
        minSdk = 24           // Минимальная версия Android для запуска
        targetSdk = 35        // Версия, под которую оптимизировано приложение
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
    }
}

dependencies {
    // Compose BOM — управляет всеми версиями Compose-зависимостей
    val composeBom = platform("androidx.compose:compose-bom:2024.12.01")
    implementation(composeBom)

    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui-tooling-preview")

    // Жизненный цикл + ViewModel
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")

    // HTTP-клиент (Retrofit + OkHttp) — для API-запросов к OpenRouter
    implementation("com.squareup.retrofit2:retrofit:2.11.0")
    implementation("com.squareup.retrofit2:converter-gson:2.11.0")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.squareup.okhttp3:logging-interceptor:4.12.0")

    // Debug-инструменты
    debugImplementation("androidx.compose.ui:ui-tooling")
}
```

> **Разница между compileSdk, minSdk, targetSdk:**
> - `compileSdk` — версия SDK, _против которой_ компилируешь. Ставь последнюю.
> - `minSdk` — минимальная версия Android, где приложение _запустится_.
> - `targetSdk` — версия, _под которую_ приложение протестировано. Android применяет compatibility-режим на более новых версиях.

### 5.2 `AndroidManifest.xml`

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <!-- Разрешения, которые нужны приложению -->
    <uses-permission android:name="android.permission.INTERNET" />

    <application
        android:allowBackup="true"
        android:label="Translator"
        android:supportsRtl="true"
        android:theme="@style/Theme.Translator">

        <!-- Объявление Activity — точки входа -->
        <activity
            android:name=".MainActivity"
            android:exported="true"
            android:windowSoftInputMode="adjustResize">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>

    </application>
</manifest>
```

### 5.3 `MainActivity.kt` — шаблон с Compose

```kotlin
package com.translator.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            TranslatorTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    TranslatorScreen()
                }
            }
        }
    }
}

@Composable
fun TranslatorScreen() {
    var sourceText by remember { mutableStateOf("") }
    var translatedText by remember { mutableStateOf("") }

    Column(modifier = Modifier.padding(16.dp)) {
        // Поле ввода
        OutlinedTextField(
            value = sourceText,
            onValueChange = { sourceText = it },
            label = { Text("Текст для перевода") },
            modifier = Modifier.fillMaxWidth()
        )

        Spacer(modifier = Modifier.height(16.dp))

        // Кнопка перевода
        Button(
            onClick = {
                // TODO: вызов API перевода
                translatedText = "Переведённый текст..."
            },
            modifier = Modifier.fillMaxWidth()
        ) {
            Text("Перевести")
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Результат
        Text(
            text = translatedText,
            style = MaterialTheme.typography.bodyLarge
        )
    }
}
```

## 6. Специфика порта Translator

Исходный проект — clipboard-переводчик на Rust + GTK3. При портировании на Android нужно реализовать:

| Возможность | Реализация на Android |
|---|---|
| Чтение/запись буфера обмена | `ClipboardManager` + `ClipData` |
| Детекция языка | lingua-rs работает через JNI/UniFFI, либо использовать `androidx.ml:language-detection` |
| Перевод через OpenRouter API | Retrofit + OkHttp (POST на `https://openrouter.ai/api/v1/chat/completions`) |
| API-ключ из конфига | `SharedPreferences` или `DataStore` |
| Выбор языков | `DropdownMenu` в Compose Material3 |

### 6.1 Работа с буфером обмена (Clipboard)

```kotlin
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context

fun copyToClipboard(context: Context, text: String) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    val clip = ClipData.newPlainText("translation", text)
    clipboard.setPrimaryClip(clip)
}

fun getClipboardText(context: Context): String? {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    return clipboard.primaryClip?.getItemAt(0)?.text?.toString()
}
```

В Jetpack Compose есть также `LocalClipboardManager`:
```kotlin
val clipboardManager = LocalClipboardManager.current
clipboardManager.setText(AnnotatedString("текст"))
val text = clipboardManager.getText()?.text
```

### 6.2 HTTP-запросы к OpenRouter API

```kotlin
import retrofit2.http.*

// Data classes для запроса/ответа
data class ChatRequest(
    val model: String,
    val messages: List<Message>
)

data class Message(
    val role: String,
    val content: String
)

data class ChatResponse(
    val choices: List<Choice>
)

data class Choice(
    val message: Message
)

// Интерфейс API
interface OpenRouterApi {
    @POST("api/v1/chat/completions")
    suspend fun translate(
        @Header("Authorization") auth: String,
        @Body request: ChatRequest
    ): ChatResponse
}

// Создание клиента (синглтон)
object ApiClient {
    private val okHttpClient = OkHttpClient.Builder()
        .addInterceptor { chain ->
            val request = chain.request().newBuilder()
                .addHeader("HTTP-Referer", "https://translator.app")
                .build()
            chain.proceed(request)
        }
        .build()

    val api: OpenRouterApi = Retrofit.Builder()
        .baseUrl("https://openrouter.ai/")
        .client(okHttpClient)
        .addConverterFactory(GsonConverterFactory.create())
        .build()
        .create(OpenRouterApi::class.java)
}
```

## 7. Сборка и запуск (CLI)

Проект полностью управляется через `make`:

```bash
cd android/
make setup           # Первый запуск: SDK + проект + gradlew
make build           # Собрать debug APK
make install         # Собрать и установить на устройство
make release         # Собрать release (unsigned)
```

### Что даёт `make`

| Команда | Что делает |
|---|---|
| `make setup` | Всё сразу: SDK + проект + gradlew |
| `make sdk` | Скачать command-line tools и установить SDK |
| `make project` | Сгенерировать gradlew и local.properties |
| `make build` | `./gradlew assembleDebug` — APK в `app/build/outputs/apk/debug/` |
| `make release` | `./gradlew assembleRelease` |
| `make install` | Build + `adb install` на подключённое устройство |
| `make avd` | Создать эмулятор (Pixel 7, API 35) |
| `make emu` | Запустить эмулятор |
| `make clean` | `./gradlew clean` |
| `make nuke` | Clean + удалить SDK и gradlew |
| `make help` | Показать все цели |

### Ручная установка APK
```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

### Если нет `adb` в системе
`make sdk` ставит adb в `sdk/platform-tools/adb`. Добавь в PATH:
```bash
export PATH="$PWD/sdk/platform-tools:$PATH"
```

## 8. Полезные ссылки

- **Официальная документация**: https://developer.android.com
- **Jetpack Compose Quickstart**: https://developer.android.com/develop/ui/compose/setup
- **Codelab «Create your first app»**: https://developer.android.com/codelabs/basic-android-kotlin-compose-first-app
- **Android Basics with Compose (курс)**: https://developer.android.com/courses/android-basics-compose/course
- **Material3 для Compose**: https://developer.android.com/develop/ui/compose/designsystems/material3
- **Архитектура Android-приложений**: https://developer.android.com/topic/architecture
- **Retrofit**: https://square.github.io/retrofit/
- **Clipboard в Compose**: https://developer.android.com/develop/ui/compose/touch-input/copy-and-paste
- **Now in Android (reference app)**: https://github.com/android/nowinandroid

## 9. Что дальше

Проект уже готов к сборке (`make build` даст APK). Осталось:

1. ⏳ Интегрировать OpenRouter API через Retrofit (см. раздел 6.2)
2. ⏳ Реализовать автоматическое определение языка (ML Kit или lingua-rs JNI)
3. ⏳ Добавить функциональность буфера обмена (copy & close)
4. ⏳ Настроить DataStore/SharedPreferences для конфига (API ключ, языки)
5. ⏳ Выбор языка через Compose DropdownMenu
