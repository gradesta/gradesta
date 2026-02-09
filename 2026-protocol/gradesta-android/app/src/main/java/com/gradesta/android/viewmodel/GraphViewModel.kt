package com.gradesta.android.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.gradesta.android.data.model.Direction
import com.gradesta.android.data.model.GraphState
import com.gradesta.android.data.model.Vertex
import com.gradesta.android.data.repository.ConnectionState
import com.gradesta.android.data.repository.GraphRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import javax.inject.Inject

/**
 * ViewModel for the main graph view.
 */
@HiltViewModel
class GraphViewModel @Inject constructor(
    private val repository: GraphRepository
) : ViewModel() {

    val graphState: StateFlow<GraphState> = repository.graphState
    val connectionState: StateFlow<ConnectionState> = repository.connectionState
    val statusMessage: StateFlow<String?> = repository.statusMessage
    val lastNavDirection: StateFlow<Int> = repository.lastNavDirection

    private val _serverUrl = MutableStateFlow("ws://localhost:8080")
    val serverUrl: StateFlow<String> = _serverUrl.asStateFlow()

    private val _landmark = MutableStateFlow("/")
    val landmark: StateFlow<String> = _landmark.asStateFlow()

    private val _creationMode = MutableStateFlow(CreationMode.AUDIO)
    val creationMode: StateFlow<CreationMode> = _creationMode.asStateFlow()

    private val _isToolbarExpanded = MutableStateFlow(false)
    val isToolbarExpanded: StateFlow<Boolean> = _isToolbarExpanded.asStateFlow()

    /**
     * Combined UI state for the main screen.
     */
    val uiState: StateFlow<MainUiState> = combine(
        graphState,
        connectionState,
        lastNavDirection
    ) { graph, connection, direction ->
        MainUiState(
            graphState = graph,
            connectionState = connection,
            lastDirection = direction,
            currentVertex = graph.currentVertex,
            neighbors = getNeighbors(graph)
        )
    }.stateIn(
        viewModelScope,
        SharingStarted.WhileSubscribed(5000),
        MainUiState()
    )

    private fun getNeighbors(state: GraphState): Neighbors {
        val current = state.currentVertex ?: return Neighbors()
        return Neighbors(
            west = state.getVertex(current.westNeighbor.toULong()),
            east = state.getVertex(current.eastNeighbor.toULong()),
            north = state.getVertex(current.northNeighbor.toULong()),
            south = state.getVertex(current.southNeighbor.toULong())
        )
    }

    fun setServerUrl(url: String) {
        _serverUrl.value = url
    }

    fun setLandmark(landmark: String) {
        _landmark.value = landmark
    }

    fun connect() {
        repository.connect(_serverUrl.value, _landmark.value)
    }

    fun disconnect() {
        repository.disconnect()
    }

    fun navigateWest() {
        repository.navigate(Direction.WEST)
    }

    fun navigateEast() {
        repository.navigate(Direction.EAST)
    }

    fun navigateNorth() {
        repository.navigate(Direction.NORTH)
    }

    fun navigateSouth() {
        repository.navigate(Direction.SOUTH)
    }

    fun navigateUp() {
        repository.navigate(Direction.UP)
    }

    fun navigateDown() {
        repository.navigate(Direction.DOWN)
    }

    fun navigateTo(vertexId: ULong) {
        repository.navigateTo(vertexId)
    }

    fun setLastDirection(direction: Int) {
        repository.setLastDirection(direction)
    }

    fun setCreationMode(mode: CreationMode) {
        _creationMode.value = mode
    }

    fun toggleToolbar() {
        _isToolbarExpanded.value = !_isToolbarExpanded.value
    }

    fun expandToolbar() {
        _isToolbarExpanded.value = true
    }

    fun collapseToolbar() {
        _isToolbarExpanded.value = false
    }

    fun createTextVertex(text: String) {
        repository.createVertex("text/plain", text.toByteArray(Charsets.UTF_8))
    }

    fun createAudioVertex(audioData: ByteArray) {
        repository.createVertex("audio/ogg", audioData)
    }

    fun createImageVertex(imageData: ByteArray, mime: String = "image/jpeg") {
        repository.createVertex(mime, imageData)
    }

    fun editCurrentVertex(text: String) {
        val vertexId = graphState.value.currentVertexId ?: return
        repository.setVertexLabel(
            vertexId,
            0u,
            "text/plain",
            text.toByteArray(Charsets.UTF_8)
        )
    }

    fun deleteCurrentVertex() {
        val vertexId = graphState.value.currentVertexId ?: return
        repository.deleteVertex(vertexId)
    }
}

data class MainUiState(
    val graphState: GraphState = GraphState(),
    val connectionState: ConnectionState = ConnectionState.Disconnected,
    val lastDirection: Int = Direction.SOUTH,
    val currentVertex: Vertex? = null,
    val neighbors: Neighbors = Neighbors()
)

data class Neighbors(
    val west: Vertex? = null,
    val east: Vertex? = null,
    val north: Vertex? = null,
    val south: Vertex? = null
)

enum class CreationMode {
    AUDIO,
    PHOTO,
    TEXT,
    ATTACHMENT
}
