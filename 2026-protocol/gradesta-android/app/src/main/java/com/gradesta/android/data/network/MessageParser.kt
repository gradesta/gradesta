package com.gradesta.android.data.network

import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Parser for Gradesta binary protocol messages.
 */
object MessageParser {

    /**
     * Parse a binary message from the server into a ServerEvent.
     */
    fun parseServerMessage(data: ByteArray): ServerEvent? {
        if (data.isEmpty()) return null

        return try {
            when (data[0]) {
                GradestaProtocol.MSG_SERVER_SET_CONTEXT -> parseSetContext(data)
                GradestaProtocol.MSG_SERVER_SET_VERTEX_LABEL -> parseSetVertexLabel(data)
                GradestaProtocol.MSG_SERVER_SET_EDGES -> parseSetEdges(data)
                GradestaProtocol.MSG_SERVER_LOG -> parseLog(data)
                GradestaProtocol.MSG_SERVER_REQUEST_IDENTIFICATION -> parseRequestIdentification(data)
                else -> null
            }
        } catch (e: Exception) {
            null
        }
    }

    private fun parseSetContext(data: ByteArray): ServerEvent.SetContext {
        // Format: [type:1][action_id:8][uri:...]
        val buffer = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN)
        buffer.get() // skip type
        buffer.getLong() // skip action_id
        val uriBytes = ByteArray(buffer.remaining())
        buffer.get(uriBytes)
        return ServerEvent.SetContext(String(uriBytes, Charsets.UTF_8))
    }

    private fun parseSetVertexLabel(data: ByteArray): ServerEvent.SetVertexLabel {
        // Format: [type:1][action_id:8][vertex_id:8][layer:4][mime:null-terminated][data:...]
        val buffer = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN)
        buffer.get() // skip type
        buffer.getLong() // skip action_id
        val vertexId = buffer.getLong().toULong()
        val layer = buffer.getInt().toUInt()

        // Read null-terminated MIME string
        val mimeBuilder = StringBuilder()
        while (buffer.hasRemaining()) {
            val b = buffer.get()
            if (b == 0.toByte()) break
            mimeBuilder.append(b.toInt().toChar())
        }
        val mime = mimeBuilder.toString()

        // Rest is data
        val labelData = ByteArray(buffer.remaining())
        buffer.get(labelData)

        return ServerEvent.SetVertexLabel(vertexId, layer, mime, labelData)
    }

    private fun parseSetEdges(data: ByteArray): ServerEvent.SetEdges {
        // Format: [type:1][action_id:8][vertex_id:8][edges:6*8][edit_mask:1?]
        val buffer = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN)
        buffer.get() // skip type
        buffer.getLong() // skip action_id
        val vertexId = buffer.getLong().toULong()

        val edges = LongArray(6) { buffer.getLong() }

        val editMask = if (buffer.hasRemaining()) buffer.get() else 0x7F.toByte()

        return ServerEvent.SetEdges(vertexId, edges, editMask)
    }

    private fun parseLog(data: ByteArray): ServerEvent.Log {
        // Format: [type:1][action_id:8][status:4][vertex_id:8][message:...]
        val buffer = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN)
        buffer.get() // skip type
        val actionId = buffer.getLong().toULong()
        val status = buffer.getInt().toUInt()
        val vertexId = buffer.getLong().toULong()
        val messageBytes = ByteArray(buffer.remaining())
        buffer.get(messageBytes)
        val message = String(messageBytes, Charsets.UTF_8)

        return ServerEvent.Log(actionId, status, vertexId, message)
    }

    private fun parseRequestIdentification(data: ByteArray): ServerEvent.RequestIdentification {
        // Format: [type:1][action_id:8][nonce:32][timestamp:8][reason:...]
        val buffer = ByteBuffer.wrap(data).order(ByteOrder.BIG_ENDIAN)
        buffer.get() // skip type
        val actionId = buffer.getLong().toULong()

        val nonce = ByteArray(32)
        buffer.get(nonce)

        val timestamp = buffer.getLong().toULong()

        val reasonBytes = ByteArray(buffer.remaining())
        buffer.get(reasonBytes)
        val reason = String(reasonBytes, Charsets.UTF_8)

        return ServerEvent.RequestIdentification(actionId, nonce, timestamp, reason)
    }

    /**
     * Serialize a client command to binary format.
     */
    fun serializeCommand(command: ClientCommand): ByteArray {
        return when (command) {
            is ClientCommand.WatchLandmark -> serializeWatchLandmark(command)
            is ClientCommand.ClickVertex -> serializeClickVertex(command)
            is ClientCommand.SetVertexLabel -> serializeSetVertexLabel(command)
            is ClientCommand.CreateVertex -> serializeCreateVertex(command)
            is ClientCommand.DeleteVertex -> serializeDeleteVertex(command)
            is ClientCommand.SetEdges -> serializeSetEdges(command)
            is ClientCommand.IdentificationResponse -> serializeIdentificationResponse(command)
            is ClientCommand.IdentificationRefused -> serializeIdentificationRefused(command)
        }
    }

    private fun serializeWatchLandmark(cmd: ClientCommand.WatchLandmark): ByteArray {
        val landmarkBytes = cmd.landmark.toByteArray(Charsets.UTF_8)
        val buffer = ByteBuffer.allocate(1 + 8 + landmarkBytes.size).order(ByteOrder.BIG_ENDIAN)
        buffer.put(GradestaProtocol.MSG_CLIENT_WATCH_LANDMARK)
        buffer.putLong(cmd.actionId.toLong())
        buffer.put(landmarkBytes)
        return buffer.array()
    }

    private fun serializeClickVertex(cmd: ClientCommand.ClickVertex): ByteArray {
        val buffer = ByteBuffer.allocate(1 + 8 + 8).order(ByteOrder.BIG_ENDIAN)
        buffer.put(GradestaProtocol.MSG_CLIENT_CLICK_VERTEX)
        buffer.putLong(cmd.actionId.toLong())
        buffer.putLong(cmd.vertexId.toLong())
        return buffer.array()
    }

    private fun serializeSetVertexLabel(cmd: ClientCommand.SetVertexLabel): ByteArray {
        val mimeBytes = cmd.mime.toByteArray(Charsets.UTF_8)
        val buffer = ByteBuffer.allocate(1 + 8 + 8 + 4 + mimeBytes.size + 1 + cmd.data.size)
            .order(ByteOrder.BIG_ENDIAN)
        buffer.put(GradestaProtocol.MSG_CLIENT_SET_VERTEX_LABEL)
        buffer.putLong(cmd.actionId.toLong())
        buffer.putLong(cmd.vertexId.toLong())
        buffer.putInt(cmd.layer.toInt())
        buffer.put(mimeBytes)
        buffer.put(0) // null terminator
        buffer.put(cmd.data)
        return buffer.array()
    }

    private fun serializeCreateVertex(cmd: ClientCommand.CreateVertex): ByteArray {
        val mimeBytes = cmd.mime.toByteArray(Charsets.UTF_8)
        val buffer = ByteBuffer.allocate(1 + 8 + 8 + 1 + 4 + mimeBytes.size + 1 + cmd.data.size)
            .order(ByteOrder.BIG_ENDIAN)
        buffer.put(GradestaProtocol.MSG_CLIENT_CREATE_VERTEX)
        buffer.putLong(cmd.actionId.toLong())
        buffer.putLong(cmd.fromVertex.toLong())
        buffer.put(cmd.direction)
        buffer.putInt(cmd.layer.toInt())
        buffer.put(mimeBytes)
        buffer.put(0) // null terminator
        buffer.put(cmd.data)
        return buffer.array()
    }

    private fun serializeDeleteVertex(cmd: ClientCommand.DeleteVertex): ByteArray {
        val buffer = ByteBuffer.allocate(1 + 8 + 8).order(ByteOrder.BIG_ENDIAN)
        buffer.put(GradestaProtocol.MSG_CLIENT_DELETE_VERTEX)
        buffer.putLong(cmd.actionId.toLong())
        buffer.putLong(cmd.vertexId.toLong())
        return buffer.array()
    }

    private fun serializeSetEdges(cmd: ClientCommand.SetEdges): ByteArray {
        val buffer = ByteBuffer.allocate(1 + 8 + 8 + 48).order(ByteOrder.BIG_ENDIAN)
        buffer.put(GradestaProtocol.MSG_CLIENT_SET_EDGES)
        buffer.putLong(cmd.actionId.toLong())
        buffer.putLong(cmd.vertexId.toLong())
        for (edge in cmd.edges) {
            buffer.putLong(edge)
        }
        return buffer.array()
    }

    private fun serializeIdentificationResponse(cmd: ClientCommand.IdentificationResponse): ByteArray {
        val urlBytes = cmd.identityUrl.toByteArray(Charsets.UTF_8)
        val buffer = ByteBuffer.allocate(1 + 8 + urlBytes.size + 1 + cmd.signature.size)
            .order(ByteOrder.BIG_ENDIAN)
        buffer.put(GradestaProtocol.MSG_CLIENT_IDENTIFICATION_RESPONSE)
        buffer.putLong(cmd.actionId.toLong())
        buffer.put(urlBytes)
        buffer.put(0) // null terminator
        buffer.put(cmd.signature)
        return buffer.array()
    }

    private fun serializeIdentificationRefused(cmd: ClientCommand.IdentificationRefused): ByteArray {
        val buffer = ByteBuffer.allocate(1 + 8).order(ByteOrder.BIG_ENDIAN)
        buffer.put(GradestaProtocol.MSG_CLIENT_IDENTIFICATION_REFUSED)
        buffer.putLong(cmd.actionId.toLong())
        return buffer.array()
    }
}
