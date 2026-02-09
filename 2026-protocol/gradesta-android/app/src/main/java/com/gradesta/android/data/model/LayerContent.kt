package com.gradesta.android.data.model

/**
 * Content for a specific layer of a vertex.
 *
 * Layers:
 * - 0: Primary content (stored in Vertex.label)
 * - 1: Transcript/alt text
 * - 2: Full resolution media
 * - 3: HTTP stream URL
 */
data class LayerContent(
    val data: ByteArray,
    val mime: String
) {
    val asString: String get() = data.decodeToString()

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as LayerContent

        if (!data.contentEquals(other.data)) return false
        if (mime != other.mime) return false

        return true
    }

    override fun hashCode(): Int {
        var result = data.contentHashCode()
        result = 31 * result + mime.hashCode()
        return result
    }
}
