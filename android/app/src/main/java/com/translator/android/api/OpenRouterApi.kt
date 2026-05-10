package com.translator.android.api

import com.translator.android.api.dto.ChatRequest
import com.translator.android.api.dto.ChatResponse
import retrofit2.http.*

interface OpenRouterApi {

    @POST("chat/completions")
    suspend fun chatCompletion(
        @Header("Authorization") auth: String,
        @Header("HTTP-Referer") referer: String = "https://translator.app",
        @Body request: ChatRequest,
    ): ChatResponse
}
