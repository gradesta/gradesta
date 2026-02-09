package com.gradesta.android.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import com.gradesta.android.data.model.Vertex
import com.gradesta.android.ui.theme.GradestaGreen
import com.gradesta.android.ui.theme.GradestaGreenLight

/**
 * Card component for displaying a vertex.
 */
@Composable
fun VertexCard(
    vertex: Vertex,
    isSelected: Boolean = false,
    modifier: Modifier = Modifier,
    onClick: () -> Unit = {}
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        border = if (isSelected) BorderStroke(3.dp, GradestaGreen) else null,
        colors = CardDefaults.cardColors(
            containerColor = if (isSelected) GradestaGreenLight else MaterialTheme.colorScheme.surface
        ),
        elevation = CardDefaults.cardElevation(
            defaultElevation = if (isSelected) 8.dp else 2.dp
        ),
        onClick = onClick
    ) {
        when {
            vertex.isText -> TextCardContent(vertex)
            vertex.isImage -> ImageCardContent(vertex)
            vertex.isAudio -> AudioCardContent(vertex)
            vertex.isVideo -> VideoCardContent(vertex)
            vertex.isPortal -> PortalCardContent(vertex)
            else -> GenericCardContent(vertex)
        }
    }
}

@Composable
private fun TextCardContent(vertex: Vertex) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(16.dp)
    ) {
        Text(
            text = vertex.labelAsString,
            style = MaterialTheme.typography.bodyLarge,
            maxLines = 6,
            overflow = TextOverflow.Ellipsis
        )
    }
}

@Composable
private fun ImageCardContent(vertex: Vertex) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(200.dp)
    ) {
        // Try layer 2 (full res) first, then layer 0
        val imageData = vertex.layers[2]?.data ?: vertex.label

        AsyncImage(
            model = imageData,
            contentDescription = "Image content",
            modifier = Modifier
                .fillMaxSize()
                .clip(RoundedCornerShape(16.dp)),
            contentScale = ContentScale.Crop
        )

        // Show transcript if available
        vertex.layers[1]?.let { transcript ->
            Text(
                text = transcript.asString,
                style = MaterialTheme.typography.bodySmall,
                color = Color.White,
                modifier = Modifier
                    .align(Alignment.BottomStart)
                    .fillMaxWidth()
                    .background(Color.Black.copy(alpha = 0.6f))
                    .padding(8.dp),
                maxLines = 2,
                overflow = TextOverflow.Ellipsis
            )
        }
    }
}

@Composable
private fun AudioCardContent(vertex: Vertex) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(16.dp)
    ) {
        // Title from transcript or default
        val title = vertex.layers[1]?.asString?.take(50) ?: "Audio"

        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis
        )

        AudioWaveform(
            modifier = Modifier
                .fillMaxWidth()
                .padding(top = 12.dp)
        )
    }
}

@Composable
private fun VideoCardContent(vertex: Vertex) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .height(180.dp)
            .background(Color.Black, RoundedCornerShape(16.dp))
    ) {
        // Show thumbnail if available
        vertex.layers[2]?.data?.let { imageData ->
            AsyncImage(
                model = imageData,
                contentDescription = "Video thumbnail",
                modifier = Modifier.fillMaxSize(),
                contentScale = ContentScale.Crop
            )
        }

        // Play button overlay
        Text(
            text = "▶",
            style = MaterialTheme.typography.headlineLarge,
            color = Color.White,
            modifier = Modifier.align(Alignment.Center)
        )

        // Title from transcript if available
        vertex.layers[1]?.let { transcript ->
            Text(
                text = transcript.asString,
                style = MaterialTheme.typography.bodySmall,
                color = Color.White,
                modifier = Modifier
                    .align(Alignment.BottomStart)
                    .fillMaxWidth()
                    .background(Color.Black.copy(alpha = 0.6f))
                    .padding(8.dp),
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }
    }
}

@Composable
private fun PortalCardContent(vertex: Vertex) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(
            text = "🔗",
            style = MaterialTheme.typography.headlineMedium
        )
        Text(
            text = vertex.labelAsString,
            style = MaterialTheme.typography.bodyMedium,
            textAlign = TextAlign.Center,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis
        )
    }
}

@Composable
private fun GenericCardContent(vertex: Vertex) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(16.dp)
    ) {
        Text(
            text = vertex.mime ?: "Unknown type",
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
        )
        Text(
            text = "${vertex.label.size} bytes",
            style = MaterialTheme.typography.bodySmall
        )
    }
}
