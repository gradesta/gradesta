package com.gradesta.android.data.network

/**
 * Protocol constants for Gradesta WebSocket communication.
 */
object GradestaProtocol {
    // Server -> Client message types
    const val MSG_SERVER_SET_CONTEXT: Byte = 0x01
    const val MSG_SERVER_SET_EDGES: Byte = 0x03
    const val MSG_SERVER_SET_VERTEX_LABEL: Byte = 0x05
    const val MSG_SERVER_LOG: Byte = 0x0F
    const val MSG_SERVER_REQUEST_IDENTIFICATION: Byte = 0x10

    // Client -> Server message types
    const val MSG_CLIENT_WATCH_LANDMARK: Byte = 0x81.toByte()
    const val MSG_CLIENT_SET_EDGES: Byte = 0x83.toByte()
    const val MSG_CLIENT_CLICK_VERTEX: Byte = 0x84.toByte()
    const val MSG_CLIENT_SET_VERTEX_LABEL: Byte = 0x85.toByte()
    const val MSG_CLIENT_CREATE_VERTEX: Byte = 0x86.toByte()
    const val MSG_CLIENT_DELETE_VERTEX: Byte = 0x87.toByte()
    const val MSG_CLIENT_IDENTIFICATION_RESPONSE: Byte = 0x90.toByte()
    const val MSG_CLIENT_IDENTIFICATION_REFUSED: Byte = 0x91.toByte()

    // Edge indices
    const val EDGE_WEST = 0
    const val EDGE_EAST = 1
    const val EDGE_NORTH = 2
    const val EDGE_SOUTH = 3
    const val EDGE_UP = 4
    const val EDGE_DOWN = 5
}

/**
 * Events received from the server.
 */
sealed class ServerEvent {
    data class SetContext(val uri: String) : ServerEvent()

    data class SetVertexLabel(
        val vertexId: ULong,
        val layer: UInt,
        val mime: String,
        val data: ByteArray
    ) : ServerEvent() {
        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (javaClass != other?.javaClass) return false
            other as SetVertexLabel
            return vertexId == other.vertexId && layer == other.layer &&
                    mime == other.mime && data.contentEquals(other.data)
        }
        override fun hashCode(): Int = 31 * vertexId.hashCode() + layer.hashCode()
    }

    data class SetEdges(
        val vertexId: ULong,
        val edges: LongArray,
        val editMask: Byte
    ) : ServerEvent() {
        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (javaClass != other?.javaClass) return false
            other as SetEdges
            return vertexId == other.vertexId && edges.contentEquals(other.edges) &&
                    editMask == other.editMask
        }
        override fun hashCode(): Int = vertexId.hashCode()
    }

    data class Log(
        val actionId: ULong,
        val status: UInt,
        val vertexId: ULong,
        val message: String
    ) : ServerEvent()

    data class RequestIdentification(
        val actionId: ULong,
        val nonce: ByteArray,
        val timestamp: ULong,
        val reason: String
    ) : ServerEvent() {
        override fun equals(other: Any?): Boolean {
            if (this === other) return true
            if (javaClass != other?.javaClass) return false
            other as RequestIdentification
            return actionId == other.actionId && nonce.contentEquals(other.nonce) &&
                    timestamp == other.timestamp && reason == other.reason
        }
        override fun hashCode(): Int = actionId.hashCode()
    }

    data class Connected(val baseUrl: String) : ServerEvent()
    data class Disconnected(val reason: String) : ServerEvent()
    data class Error(val message: String) : ServerEvent()
}

/**
 * Commands to send to the server.
 */
sealed class ClientCommand {
    data class WatchLandmark(val actionId: ULong, val landmark: String) : ClientCommand()
    data class ClickVertex(val actionId: ULong, val vertexId: ULong) : ClientCommand()
    data class SetVertexLabel(
        val actionId: ULong,
        val vertexId: ULong,
        val layer: UInt,
        val mime: String,
        val data: ByteArray
    ) : ClientCommand()
    data class CreateVertex(
        val actionId: ULong,
        val fromVertex: ULong,
        val direction: Byte,
        val layer: UInt,
        val mime: String,
        val data: ByteArray
    ) : ClientCommand()
    data class DeleteVertex(val actionId: ULong, val vertexId: ULong) : ClientCommand()
    data class SetEdges(val actionId: ULong, val vertexId: ULong, val edges: LongArray) : ClientCommand()
    data class IdentificationResponse(
        val actionId: ULong,
        val identityUrl: String,
        val signature: ByteArray
    ) : ClientCommand()
    data class IdentificationRefused(val actionId: ULong) : ClientCommand()
}
