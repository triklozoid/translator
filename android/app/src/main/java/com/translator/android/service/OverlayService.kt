package com.translator.android.service

import android.app.Service
import android.content.Context
import android.content.Intent
import android.graphics.PixelFormat
import android.os.Build
import android.os.IBinder
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.translator.android.MainActivity
import com.translator.android.TranslatorApp
import com.translator.android.data.model.Language
import com.translator.android.domain.TranslateUseCase
import com.translator.android.language.LanguageDetector
import com.translator.android.language.LanguageSelector
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first

/**
 * Foreground Service — управляет плавающим баблом и панелью перевода.
 *
 * Объединяет ClipboardService (следит за буфером) и Overlay (WindowManager).
 */
class OverlayService : Service() {

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private lateinit var windowManager: WindowManager
    private var bubbleView: View? = null
    private var panelView: View? = null
    private var isPanelShown = false

    // Состояние
    private var currentText by mutableStateOf<String?>(null)
    private var translatedText by mutableStateOf("")
    private var targetLang by mutableStateOf<Language?>(null)
    private var isTranslating by mutableStateOf(false)
    private var error by mutableStateOf<String?>(null)

    // Перетаскивание бабла
    private var initialX = 0
    private var initialY = 0
    private var initialTouchX = 0f
    private var initialTouchY = 0f

    // Clipboard listener (встроен прямо сюда)
    private var lastClipTimestamp = 0L

    @Suppress("DEPRECATION")
    override fun onCreate() {
        super.onCreate()
        NotificationHelper.createChannel(this)
        startForeground(NotificationHelper.NOTIFICATION_ID, NotificationHelper.buildNotification(this, MainActivity::class.java))

        windowManager = getSystemService(Context.WINDOW_SERVICE) as WindowManager

        // Слушаем буфер обмена
        val cm = getSystemService(Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager
        cm.addPrimaryClipChangedListener {
            val clip = cm.primaryClip ?: return@addPrimaryClipChangedListener
            if (clip.itemCount == 0) return@addPrimaryClipChangedListener
            val ts = clip.description.timestamp
            if (ts == lastClipTimestamp) return@addPrimaryClipChangedListener
            lastClipTimestamp = ts
            val text = clip.getItemAt(0).text?.toString() ?: return@addPrimaryClipChangedListener
            if (text.isBlank() || text.length > 5000) return@addPrimaryClipChangedListener
            currentText = text
            // Авто-перевод если панель уже развёрнута
            if (isPanelShown) performTranslation(text)
        }

        createBubble()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int = START_STICKY

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        removeBubble()
        removePanel()
        scope.cancel()
        super.onDestroy()
    }

    // =============================================================================
    // Bubble
    // =============================================================================
    private fun createBubble() {
        val view = ComposeView(this).apply {
            setContent {
                BubbleContent(
                    hasNewText = currentText != null,
                    onClick = { showPanel() },
                )
            }
        }

        val params = WindowManager.LayoutParams(
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.WRAP_CONTENT,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
            else WindowManager.LayoutParams.TYPE_PHONE,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE,
            PixelFormat.TRANSLUCENT,
        ).apply {
            gravity = Gravity.TOP or Gravity.START
            x = 50
            y = 400
        }

        view.setOnTouchListener(BubbleTouchListener(windowManager, params, view))
        windowManager.addView(view, params)
        bubbleView = view
    }

    private fun removeBubble() {
        bubbleView?.let { windowManager.removeView(it) }
        bubbleView = null
    }

    // =============================================================================
    // Panel
    // =============================================================================
    private fun showPanel() {
        if (isPanelShown) return
        isPanelShown = true
        removeBubble()

        val view = ComposeView(this).apply {
            setContent {
                TranslationPanelContent(
                    sourceText = currentText ?: "",
                    translatedText = translatedText,
                    isTranslating = isTranslating,
                    error = error,
                    onDismiss = { hidePanel() },
                    onCopyAndClose = {
                        val text = translatedText.ifEmpty { currentText ?: "" }
                        val cm = getSystemService(Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager
                        cm.setPrimaryClip(android.content.ClipData.newPlainText("translation", text))
                        currentText = null
                        hidePanel()
                    },
                )
            }
        }

        val displayMetrics = resources.displayMetrics
        val width = (displayMetrics.widthPixels * if (displayMetrics.widthPixels > displayMetrics.heightPixels) 0.75f else 1.0f).toInt()
        val height = (displayMetrics.heightPixels * 0.6f).toInt()

        val params = WindowManager.LayoutParams(
            width,
            height,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
            else WindowManager.LayoutParams.TYPE_PHONE,
            WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL,
            PixelFormat.TRANSLUCENT,
        ).apply {
            gravity = Gravity.CENTER
        }

        windowManager.addView(view, params)
        panelView = view

        // Авто-перевод текста из буфера
        currentText?.let { performTranslation(it) }
    }

    private fun hidePanel() {
        isPanelShown = false
        removePanel()
        createBubble()
    }

    private fun removePanel() {
        panelView?.let { windowManager.removeView(it) }
        panelView = null
    }

    // =============================================================================
    // Translation logic
    // =============================================================================
    private fun performTranslation(text: String) {
        scope.launch {
            isTranslating = true
            error = null
            translatedText = ""

            val settings = TranslatorApp.instance.settings
            val apiKey = settings.apiKeyFlow.first()
            val apiUrl = settings.apiUrlFlow.first()
            val model = settings.modelVersionFlow.first()
            val primaryLang = settings.primaryLanguageFlow.first()
            val secondaryLang = settings.secondaryLanguageFlow.first()
            val lastLang = settings.lastTargetLanguageFlow.first()

            val useCase = TranslateUseCase()
            val result = useCase.translate(
                text = text,
                targetLang = targetLang,
                apiKey = apiKey,
                apiUrl = apiUrl,
                model = model,
                primaryLang = primaryLang,
                secondaryLang = secondaryLang,
                lastLang = lastLang,
            )

            result.fold(
                onSuccess = { (translated, lang) ->
                    translatedText = translated
                    targetLang = lang
                    isTranslating = false
                },
                onFailure = { e ->
                    error = e.message
                    isTranslating = false
                },
            )
        }
    }

    // =============================================================================
    // Drag listener
    // =============================================================================
    inner class BubbleTouchListener(
        private val wm: WindowManager,
        private val layoutParams: WindowManager.LayoutParams,
        private val view: View,
    ) : View.OnTouchListener {

        override fun onTouch(v: View, event: MotionEvent): Boolean {
            when (event.action) {
                MotionEvent.ACTION_DOWN -> {
                    initialX = layoutParams.x
                    initialY = layoutParams.y
                    initialTouchX = event.rawX
                    initialTouchY = event.rawY
                    return true
                }
                MotionEvent.ACTION_MOVE -> {
                    val dx = (event.rawX - initialTouchX).toInt()
                    val dy = (event.rawY - initialTouchY).toInt()
                    layoutParams.x = initialX + dx
                    layoutParams.y = initialY + dy
                    layoutParams.flags = WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                    wm.updateViewLayout(view, layoutParams)
                    return true
                }
                MotionEvent.ACTION_UP -> {
                    // Прилипание к краю
                    val dm = resources.displayMetrics
                    if (layoutParams.x + view.width / 2 < dm.widthPixels / 2) {
                        layoutParams.x = 20
                    } else {
                        layoutParams.x = dm.widthPixels - view.width - 20
                    }
                    layoutParams.flags = WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                    wm.updateViewLayout(view, layoutParams)
                    return (Math.abs(event.rawX - initialTouchX) + Math.abs(event.rawY - initialTouchY)) < 10
                }
            }
            return false
        }
    }
}

// =============================================================================
// Composable: Bubble
// =============================================================================
@Composable
private fun BubbleContent(
    hasNewText: Boolean,
    onClick: () -> Unit,
) {
    val bgColor = if (hasNewText) MaterialTheme.colorScheme.primary
    else MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.85f)

    Box(
        modifier = Modifier
            .size(56.dp)
            .shadow(4.dp, CircleShape)
            .background(bgColor, CircleShape)
            .clickable { onClick() },
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = "🌐",
            fontSize = 22.sp,
            textAlign = TextAlign.Center,
        )
    }
}

// =============================================================================
// Composable: Translation Panel
// =============================================================================
@Composable
private fun TranslationPanelContent(
    sourceText: String,
    translatedText: String,
    isTranslating: Boolean,
    error: String?,
    onDismiss: () -> Unit,
    onCopyAndClose: () -> Unit,
) {
    Surface(
        modifier = Modifier.fillMaxSize(),
        shape = MaterialTheme.shapes.large,
        shadowElevation = 8.dp,
        color = MaterialTheme.colorScheme.surface,
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            // Заголовок
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Перевод", style = MaterialTheme.typography.titleMedium)
                TextButton(onClick = onDismiss) { Text("✕") }
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Исходный текст
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
            ) {
                Text(
                    text = sourceText.ifEmpty { "Скопируйте текст..." },
                    modifier = Modifier.padding(8.dp),
                    style = MaterialTheme.typography.bodyMedium,
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Перевод
            Card(
                modifier = Modifier.fillMaxWidth().weight(1f),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.secondaryContainer),
            ) {
                Box(modifier = Modifier.fillMaxSize().padding(8.dp)) {
                    when {
                        isTranslating -> {
                            Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                    CircularProgressIndicator(modifier = Modifier.size(24.dp))
                                    Spacer(modifier = Modifier.height(8.dp))
                                    Text("Перевожу...", style = MaterialTheme.typography.bodySmall)
                                }
                            }
                        }
                        error != null -> {
                            Text(text = error!!, color = MaterialTheme.colorScheme.error)
                        }
                        translatedText.isNotEmpty() -> {
                            Text(text = translatedText, style = MaterialTheme.typography.bodyLarge)
                        }
                        else -> {
                            Text("Перевод появится здесь", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Кнопки
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Button(
                    onClick = onCopyAndClose,
                    modifier = Modifier.weight(1f),
                    enabled = translatedText.isNotEmpty(),
                ) {
                    Text("Копировать и закрыть")
                }
                OutlinedButton(
                    onClick = onDismiss,
                    modifier = Modifier.weight(1f),
                ) {
                    Text("Свернуть")
                }
            }
        }
    }
}
