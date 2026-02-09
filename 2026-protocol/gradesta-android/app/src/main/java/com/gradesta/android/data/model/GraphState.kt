package com.gradesta.android.data.model

/**
 * Represents the current state of the graph view.
 */
data class GraphState(
    val vertices: Map<ULong, Vertex> = emptyMap(),
    val currentVertexId: ULong? = null,
    val contextUri: String? = null,
    val landmarkVertices: Map<String, List<ULong>> = emptyMap()
) {
    val currentVertex: Vertex?
        get() = currentVertexId?.let { vertices[it] }

    fun getVertex(id: ULong): Vertex? = vertices[id]

    fun getNeighbor(direction: Int): Vertex? {
        val current = currentVertex ?: return null
        val neighborId = current.edges.getOrNull(direction)?.toULong() ?: return null
        if (neighborId == 0UL) return null
        return vertices[neighborId]
    }
}

/**
 * Navigation direction constants
 */
object Direction {
    const val WEST = Vertex.EDGE_WEST
    const val EAST = Vertex.EDGE_EAST
    const val NORTH = Vertex.EDGE_NORTH
    const val SOUTH = Vertex.EDGE_SOUTH
    const val UP = Vertex.EDGE_UP
    const val DOWN = Vertex.EDGE_DOWN

    fun opposite(direction: Int): Int = when (direction) {
        WEST -> EAST
        EAST -> WEST
        NORTH -> SOUTH
        SOUTH -> NORTH
        UP -> DOWN
        DOWN -> UP
        else -> SOUTH
    }

    fun name(direction: Int): String = when (direction) {
        WEST -> "West"
        EAST -> "East"
        NORTH -> "North"
        SOUTH -> "South"
        UP -> "Up"
        DOWN -> "Down"
        else -> "Unknown"
    }
}
