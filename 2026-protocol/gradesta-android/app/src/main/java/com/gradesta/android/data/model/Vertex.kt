package com.gradesta.android.data.model

/**
 * Represents a vertex in the Gradesta graph.
 *
 * @property id Unique vertex identifier
 * @property label Layer 0 content (primary content)
 * @property mime MIME type for layer 0 content
 * @property edges Array of 6 edge connections [West, East, North, South, Up, Down]
 * @property layers Additional content layers (1=transcript, 2=full-res, 3=HTTP stream)
 * @property editMask Bitmask indicating which aspects are editable
 */
data class Vertex(
    val id: ULong,
    val label: ByteArray = byteArrayOf(),
    val mime: String? = null,
    val edges: LongArray = LongArray(6) { 0L },
    val layers: Map<Int, LayerContent> = emptyMap(),
    val editMask: Int = 0
) {
    companion object {
        const val EDGE_WEST = 0
        const val EDGE_EAST = 1
        const val EDGE_NORTH = 2
        const val EDGE_SOUTH = 3
        const val EDGE_UP = 4
        const val EDGE_DOWN = 5

        // Edit mask flags
        const val EDIT_LABEL = 0x01
        const val EDIT_EDGES = 0x02
        const val EDIT_DELETE = 0x04
    }

    val westNeighbor: Long get() = edges[EDGE_WEST]
    val eastNeighbor: Long get() = edges[EDGE_EAST]
    val northNeighbor: Long get() = edges[EDGE_NORTH]
    val southNeighbor: Long get() = edges[EDGE_SOUTH]
    val upNeighbor: Long get() = edges[EDGE_UP]
    val downNeighbor: Long get() = edges[EDGE_DOWN]

    val canEditLabel: Boolean get() = (editMask and EDIT_LABEL) != 0
    val canEditEdges: Boolean get() = (editMask and EDIT_EDGES) != 0
    val canDelete: Boolean get() = (editMask and EDIT_DELETE) != 0

    val isText: Boolean get() = mime?.startsWith("text/") == true
    val isImage: Boolean get() = mime?.startsWith("image/") == true
    val isAudio: Boolean get() = mime?.startsWith("audio/") == true
    val isVideo: Boolean get() = mime?.startsWith("video/") == true
    val isPortal: Boolean get() = mime == "text/x-gradesta-portal"

    val labelAsString: String get() = label.decodeToString()

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as Vertex

        if (id != other.id) return false
        if (!label.contentEquals(other.label)) return false
        if (mime != other.mime) return false
        if (!edges.contentEquals(other.edges)) return false
        if (layers != other.layers) return false
        if (editMask != other.editMask) return false

        return true
    }

    override fun hashCode(): Int {
        var result = id.hashCode()
        result = 31 * result + label.contentHashCode()
        result = 31 * result + (mime?.hashCode() ?: 0)
        result = 31 * result + edges.contentHashCode()
        result = 31 * result + layers.hashCode()
        result = 31 * result + editMask
        return result
    }
}
