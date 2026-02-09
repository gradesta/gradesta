package com.gradesta.android.data.network

import android.util.Log
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.receiveAsFlow
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.ByteString
import okio.ByteString.Companion.toByteString
import java.util.concurrent.TimeUnit
import javax.inject.Inject
import javax.inject.Singleton

/**
 * WebSocket client for Gradesta server communication.
 */
@Singleton
class WebSocketClient @Inject constructor() {

    companion object {
        private const val TAG = "GradestaWebSocket"
        private const val PING_INTERVAL_SECONDS = 30L
    }

    private val client = OkHttpClient.Builder()
        .pingInterval(PING_INTERVAL_SECONDS, TimeUnit.SECONDS)
        .readTimeout(0, TimeUnit.MILLISECONDS) // No timeout for WebSocket
        .build()

    private var webSocket: WebSocket? = null
    private val _events = Channel<ServerEvent>(Channel.BUFFERED)
    val events: Flow<ServerEvent> = _events.receiveAsFlow()

    private var baseUrl: String? = null

    /**
     * Connect to a Gradesta server.
     *
     * @param serverUrl The WebSocket URL (e.g., ws://localhost:8080)
     * @param landmark The initial landmark to watch (e.g., "/")
     */
    fun connect(serverUrl: String, landmark: String) {
        disconnect()

        val url = buildUrl(serverUrl, landmark)
        Log.d(TAG, "Connecting to: $url")

        val request = Request.Builder()
            .url(url)
            .build()

        baseUrl = serverUrl

        webSocket = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                Log.d(TAG, "Connected")
                _events.trySend(ServerEvent.Connected(serverUrl))

                // Send initial WatchLandmark
                sendCommand(ClientCommand.WatchLandmark(0UL, landmark))
            }

            override fun onMessage(webSocket: WebSocket, bytes: ByteString) {
                val data = bytes.toByteArray()
                MessageParser.parseServerMessage(data)?.let { event ->
                    _events.trySend(event)
                }
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                // Protocol uses binary messages, not text
                Log.w(TAG, "Unexpected text message: $text")
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                Log.d(TAG, "Closing: $code $reason")
                webSocket.close(1000, null)
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                Log.d(TAG, "Closed: $code $reason")
                _events.trySend(ServerEvent.Disconnected(reason.ifEmpty { "Connection closed" }))
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                Log.e(TAG, "Failure", t)
                _events.trySend(ServerEvent.Error(t.message ?: "Connection failed"))
                _events.trySend(ServerEvent.Disconnected(t.message ?: "Connection failed"))
            }
        })
    }

    /**
     * Disconnect from the server.
     */
    fun disconnect() {
        webSocket?.close(1000, "User disconnect")
        webSocket = null
        baseUrl = null
    }

    /**
     * Send a command to the server.
     */
    fun sendCommand(command: ClientCommand) {
        val ws = webSocket
        if (ws == null) {
            Log.w(TAG, "Cannot send command: not connected")
            return
        }

        val data = MessageParser.serializeCommand(command)
        val sent = ws.send(data.toByteString())

        if (sent) {
            Log.d(TAG, "Sent command: ${command::class.simpleName}")
        } else {
            Log.w(TAG, "Failed to send command: ${command::class.simpleName}")
        }
    }

    /**
     * Watch a new landmark.
     */
    fun watchLandmark(actionId: ULong, landmark: String) {
        sendCommand(ClientCommand.WatchLandmark(actionId, landmark))
    }

    /**
     * Click/activate a vertex.
     */
    fun clickVertex(actionId: ULong, vertexId: ULong) {
        sendCommand(ClientCommand.ClickVertex(actionId, vertexId))
    }

    /**
     * Create a new vertex.
     */
    fun createVertex(
        actionId: ULong,
        fromVertex: ULong,
        direction: Int,
        layer: UInt,
        mime: String,
        data: ByteArray
    ) {
        sendCommand(ClientCommand.CreateVertex(
            actionId,
            fromVertex,
            direction.toByte(),
            layer,
            mime,
            data
        ))
    }

    /**
     * Set vertex label content.
     */
    fun setVertexLabel(
        actionId: ULong,
        vertexId: ULong,
        layer: UInt,
        mime: String,
        data: ByteArray
    ) {
        sendCommand(ClientCommand.SetVertexLabel(actionId, vertexId, layer, mime, data))
    }

    /**
     * Delete a vertex.
     */
    fun deleteVertex(actionId: ULong, vertexId: ULong) {
        sendCommand(ClientCommand.DeleteVertex(actionId, vertexId))
    }

    val isConnected: Boolean
        get() = webSocket != null

    private fun buildUrl(serverUrl: String, landmark: String): String {
        // Ensure URL has ws:// or wss:// prefix
        val normalizedUrl = when {
            serverUrl.startsWith("ws://") || serverUrl.startsWith("wss://") -> serverUrl
            serverUrl.startsWith("http://") -> serverUrl.replace("http://", "ws://")
            serverUrl.startsWith("https://") -> serverUrl.replace("https://", "wss://")
            else -> "ws://$serverUrl"
        }

        // Add landmark as query parameter if not already in path
        return if (normalizedUrl.contains("?")) {
            "$normalizedUrl&landmark=$landmark"
        } else {
            "$normalizedUrl?landmark=$landmark"
        }
    }
}
