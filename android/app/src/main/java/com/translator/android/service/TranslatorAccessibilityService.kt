package com.translator.android.service

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.AccessibilityServiceInfo
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import com.translator.android.TranslatorApp
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/**
 * Опциональный Accessibility Service.
 * Захватывает выделенный текст в любом приложении.
 *
 * Пользователь включает вручную: Settings → Accessibility → Translator.
 */
class TranslatorAccessibilityService : AccessibilityService() {

    override fun onServiceConnected() {
        val info = AccessibilityServiceInfo().apply {
            eventTypes = AccessibilityEvent.TYPE_VIEW_TEXT_SELECTION_CHANGED
            feedbackType = AccessibilityServiceInfo.FEEDBACK_GENERIC
            notificationTimeout = 100
            flags = AccessibilityServiceInfo.DEFAULT
        }
        setServiceInfo(info)
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent) {
        if (event.eventType != AccessibilityEvent.TYPE_VIEW_TEXT_SELECTION_CHANGED) return

        val source: AccessibilityNodeInfo = event.source ?: return
        val selected = try {
            val text = source.text ?: return
            val start = source.textSelectionStart
            val end = source.textSelectionEnd
            if (start < 0 || end < 0 || start >= end) return
            text.substring(start, end.coerceAtMost(text.length))
        } catch (e: Exception) {
            null
        } ?: return

        if (selected.isNotBlank() && selected.length <= 5000) {
            // Сохраняем в DataStore — бабл подхватит через flow
            CoroutineScope(Dispatchers.IO).launch {
                TranslatorApp.instance.settings.setClipboardText(selected)
            }
        }
    }

    override fun onInterrupt() {
        // Не требуется
    }
}
