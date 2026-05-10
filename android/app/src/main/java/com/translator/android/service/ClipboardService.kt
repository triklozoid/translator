package com.translator.android.service

import android.app.Service
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.os.IBinder
import com.translator.android.MainActivity
import com.translator.android.TranslatorApp
import kotlinx.coroutines.*

/**
 * Foreground Service — слушает буфер обмена.
 *
 * При каждом копировании текста сохраняет его в DataStore.
 * На Android 10+ работает только в foreground.
 */
class ClipboardService : Service() {

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private lateinit var clipboardManager: ClipboardManager
    private var lastClipTimestamp = 0L

    private val clipboardListener = ClipboardManager.OnPrimaryClipChangedListener {
        val clip = clipboardManager.primaryClip ?: return@OnPrimaryClipChangedListener
        if (clip.itemCount == 0) return@OnPrimaryClipChangedListener

        // Защита от двойного срабатывания
        val ts = clip.description.timestamp
        if (ts == lastClipTimestamp) return@OnPrimaryClipChangedListener
        lastClipTimestamp = ts

        val text = clip.getItemAt(0).text?.toString() ?: return@OnPrimaryClipChangedListener
        if (text.isBlank()) return@OnPrimaryClipChangedListener
        if (text.length > 5000) return@OnPrimaryClipChangedListener  // Слишком длинный

        scope.launch {
            TranslatorApp.instance.settings.setClipboardText(text)
        }
    }

    override fun onCreate() {
        super.onCreate()
        NotificationHelper.createChannel(this)
        startForeground(NotificationHelper.NOTIFICATION_ID, NotificationHelper.buildNotification(this, MainActivity::class.java))
        clipboardManager = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboardManager.addPrimaryClipChangedListener(clipboardListener)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        return START_STICKY  // Авто-перезапуск если убит системой
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        clipboardManager.removePrimaryClipChangedListener(clipboardListener)
        scope.cancel()
        super.onDestroy()
    }
}
