package com.translator.android.api.dto

import com.google.gson.annotations.SerializedName

data class ChatRequest(
    val model: String,
    val messages: List<Message>,
    @SerializedName("max_tokens") val maxTokens: Int = 4096,
)

data class Message(
    val role: String,
    val content: String,
)

data class ChatResponse(
    val choices: List<Choice>?,
    val content: String? = null,
    val usage: Usage? = null,
)

data class Choice(
    val message: Message?,
    @SerializedName("finish_reason") val finishReason: String? = null,
)

data class Usage(
    @SerializedName("prompt_tokens") val promptTokens: Int = 0,
    @SerializedName("completion_tokens") val completionTokens: Int = 0,
    @SerializedName("total_tokens") val totalTokens: Int = 0,
)
