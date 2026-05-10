package com.translator.android

import android.app.Application
import com.translator.android.data.SettingsDataStore

class TranslatorApp : Application() {
    lateinit var settings: SettingsDataStore
        private set

    override fun onCreate() {
        super.onCreate()
        instance = this
        settings = SettingsDataStore(this)
    }

    companion object {
        lateinit var instance: TranslatorApp
            private set
    }
}
