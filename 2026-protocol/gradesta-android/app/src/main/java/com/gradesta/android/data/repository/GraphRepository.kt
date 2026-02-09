package com.gradesta.android.data.repository

import com.gradesta.android.data.model.Direction
import com.gradesta.android.data.model.GraphState
import com.gradesta.android.data.model.LayerContent
import com.gradesta.android.data.model.Vertex
import com.gradesta.android.data.network.ClientCommand
import com.gradesta.android.data.network.ServerEvent
import com.gradesta.android.data.network.WebSocketClient
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.util.concurrent.atomic.AtomicLong
import javax.inject.Inject
import javax.inject.Singleton

/**
 * Repository for managing graph state and server communication.
 */
@Singleton
class GraphRepository @Inject constructor(
    private val webSocketClient: WebSocketClient
) {
    private val scope = CoroutineScope(Dispatchers.Default)

    private val _graphState = MutableStateFlow(GraphState())
    val graphState: StateFlow<GraphState> = _graphState.asStateFlow()

    private val _connectionState = MutableStateFlow(ConnectionState.Disconnected)
    val connectionState: StateFlow<ConnectionState> = _connectionState.asStateFlow()

    private val _statusMessage = MutableStateFlow<String?>(null)
    val statusMessage: StateFlow<String?> = _statusMessage.asStateFlow()

    private val _lastNavDirection = MutableStateFlow(Direction.SOUTH)
    val lastNavDirection: StateFlow<Int> = _lastNavDirection.asStateFlow()

    private val actionIdCounter = AtomicLong(Long.MAX_VALUE)

    init {
        // Collect server events
        scope.launch {
            webSocketClient.events.collect { event ->
                handleServerEvent(event)
            }
        }
    }

    private fun nextActionId(): ULong = actionIdCounter.decrementAndGet().toULong()

    private fun handleServerEvent(event: ServerEvent) {
        when (event) {
            is ServerEvent.Connected -> {
                _connectionState.value = ConnectionState.Connected
                _statusMessage.value = "Connected"
            }

            is ServerEvent.Disconnected -> {
                _connectionState.value = ConnectionState.Disconnected
                _statusMessage.value = event.reason
            }

            is ServerEvent.Error -> {
                _statusMessage.value = "Error: ${event.message}"
            }

            is ServerEvent.SetContext -> {
                _graphState.value = _graphState.value.copy(
                    contextUri = event.uri
                )
                _statusMessage.value = "Viewing: ${event.uri}"
            }

            is ServerEvent.SetVertexLabel -> {
                val currentState = _graphState.value
                val vertices = currentState.vertices.toMutableMap()

                val existing = vertices[event.vertexId] ?: Vertex(id = event.vertexId)

                val updated = if (event.layer == 0u) {
                    existing.copy(
                        label = event.data,
                        mime = event.mime
                    )
                } else {
                    val layers = existing.layers.toMutableMap()
                    layers[event.layer.toInt()] = LayerContent(event.data, event.mime)
                    existing.copy(layers = layers)
                }

                vertices[event.vertexId] = updated

                // Set as current vertex if none set
                val currentVertexId = currentState.currentVertexId ?: event.vertexId

                _graphState.value = currentState.copy(
                    vertices = vertices,
                    currentVertexId = currentVertexId
                )
            }

            is ServerEvent.SetEdges -> {
                val currentState = _graphState.value
                val vertices = currentState.vertices.toMutableMap()

                // Check if this is a deletion (all edges = 0, editMask = 0)
                val isDeleted = event.edges.all { it == 0L } && event.editMask == 0.toByte()

                if (isDeleted && vertices.containsKey(event.vertexId)) {
                    // Remove vertex
                    vertices.remove(event.vertexId)

                    // If this was current vertex, navigate to another
                    val newCurrentId = if (currentState.currentVertexId == event.vertexId) {
                        vertices.keys.firstOrNull()
                    } else {
                        currentState.currentVertexId
                    }

                    _graphState.value = currentState.copy(
                        vertices = vertices,
                        currentVertexId = newCurrentId
                    )
                } else {
                    val existing = vertices[event.vertexId] ?: Vertex(id = event.vertexId)
                    vertices[event.vertexId] = existing.copy(
                        edges = event.edges,
                        editMask = event.editMask.toInt()
                    )

                    _graphState.value = currentState.copy(vertices = vertices)
                }
            }

            is ServerEvent.Log -> {
                if (event.status != 0u) {
                    _statusMessage.value = event.message
                }
            }

            is ServerEvent.RequestIdentification -> {
                // TODO: Handle authentication requests
                _statusMessage.value = "Authentication requested: ${event.reason}"
            }
        }
    }

    fun connect(serverUrl: String, landmark: String) {
        _connectionState.value = ConnectionState.Connecting
        _graphState.value = GraphState() // Reset state
        webSocketClient.connect(serverUrl, landmark)
    }

    fun disconnect() {
        webSocketClient.disconnect()
        _graphState.value = GraphState()
        _connectionState.value = ConnectionState.Disconnected
    }

    fun navigateTo(vertexId: ULong) {
        val currentState = _graphState.value
        if (!currentState.vertices.containsKey(vertexId)) return

        _graphState.value = currentState.copy(currentVertexId = vertexId)
        webSocketClient.clickVertex(nextActionId(), vertexId)
    }

    fun navigate(direction: Int): Boolean {
        val currentState = _graphState.value
        val currentVertex = currentState.currentVertex ?: return false

        val neighborId = currentVertex.edges.getOrNull(direction)?.toULong() ?: return false
        if (neighborId == 0UL) return false

        _lastNavDirection.value = direction

        if (currentState.vertices.containsKey(neighborId)) {
            _graphState.value = currentState.copy(currentVertexId = neighborId)
            webSocketClient.clickVertex(nextActionId(), neighborId)
            return true
        }

        return false
    }

    fun createVertex(mime: String, data: ByteArray) {
        val currentState = _graphState.value
        val currentVertexId = currentState.currentVertexId ?: return

        webSocketClient.createVertex(
            actionId = nextActionId(),
            fromVertex = currentVertexId,
            direction = _lastNavDirection.value,
            layer = 0u,
            mime = mime,
            data = data
        )
    }

    fun setVertexLabel(vertexId: ULong, layer: UInt, mime: String, data: ByteArray) {
        webSocketClient.setVertexLabel(nextActionId(), vertexId, layer, mime, data)
    }

    fun deleteVertex(vertexId: ULong) {
        webSocketClient.deleteVertex(nextActionId(), vertexId)
    }

    fun setLastDirection(direction: Int) {
        _lastNavDirection.value = direction
    }
}

enum class ConnectionState {
    Disconnected,
    Connecting,
    Connected
}
