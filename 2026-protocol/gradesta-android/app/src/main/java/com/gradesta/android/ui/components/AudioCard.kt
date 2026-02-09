package com.gradesta.android.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.unit.dp
import com.gradesta.android.ui.theme.GradestaGreen
import kotlin.math.sin

/**
 * Audio waveform visualization component.
 */
@Composable
fun AudioWaveform(
    modifier: Modifier = Modifier,
    color: Color = GradestaGreen,
    isPlaying: Boolean = false,
    progress: Float = 0f
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .height(48.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        // Play button
        IconButton(
            onClick = { /* TODO: Handle play/pause */ },
            modifier = Modifier
                .size(48.dp)
                .background(GradestaGreen, CircleShape)
        ) {
            Icon(
                imageVector = Icons.Default.PlayArrow,
                contentDescription = "Play",
                tint = Color.White,
                modifier = Modifier.size(28.dp)
            )
        }

        // Waveform visualization
        Canvas(
            modifier = Modifier
                .weight(1f)
                .height(40.dp)
                .padding(horizontal = 4.dp)
        ) {
            val width = size.width
            val height = size.height
            val centerY = height / 2

            val barCount = 40
            val barWidth = 3.dp.toPx()
            val spacing = (width - barCount * barWidth) / (barCount - 1)

            for (i in 0 until barCount) {
                val x = i * (barWidth + spacing) + barWidth / 2

                // Generate pseudo-random height based on position
                val seed = i * 1234.5f
                val randomHeight = (sin(seed) * 0.5f + 0.5f) * 0.8f + 0.2f
                val barHeight = height * randomHeight

                // Color based on progress
                val isPlayed = i.toFloat() / barCount < progress
                val barColor = if (isPlayed) color else color.copy(alpha = 0.4f)

                drawLine(
                    color = barColor,
                    start = Offset(x, centerY - barHeight / 2),
                    end = Offset(x, centerY + barHeight / 2),
                    strokeWidth = barWidth,
                    cap = StrokeCap.Round
                )
            }
        }
    }
}

/**
 * Audio player controls for expanded view.
 */
@Composable
fun AudioPlayerControls(
    isPlaying: Boolean,
    progress: Float,
    duration: Long,
    onPlayPause: () -> Unit,
    onSeek: (Float) -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .padding(16.dp)
    ) {
        // TODO: Implement full audio player controls
        AudioWaveform(
            isPlaying = isPlaying,
            progress = progress
        )
    }
}
