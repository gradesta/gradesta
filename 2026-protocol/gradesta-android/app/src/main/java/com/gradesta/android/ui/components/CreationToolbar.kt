package com.gradesta.android.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandHorizontally
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkHorizontally
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AttachFile
import androidx.compose.material.icons.filled.CameraAlt
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material.icons.filled.TextFields
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.FloatingActionButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import com.gradesta.android.ui.theme.GradestaGreen
import com.gradesta.android.ui.theme.GradestaGreenLight
import com.gradesta.android.viewmodel.CreationMode

/**
 * Bottom toolbar for content creation mode selection.
 */
@Composable
fun CreationToolbar(
    selectedMode: CreationMode,
    isExpanded: Boolean,
    onModeSelected: (CreationMode) -> Unit,
    onPrimaryAction: () -> Unit,
    onToggleExpand: () -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .fillMaxWidth()
            .padding(16.dp),
        contentAlignment = Alignment.Center
    ) {
        AnimatedVisibility(
            visible = isExpanded,
            enter = fadeIn() + expandHorizontally(),
            exit = fadeOut() + shrinkHorizontally()
        ) {
            // Expanded toolbar
            Surface(
                shape = RoundedCornerShape(32.dp),
                color = MaterialTheme.colorScheme.surface,
                shadowElevation = 8.dp
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // Microphone button (primary)
                    ModeButton(
                        icon = Icons.Default.Mic,
                        contentDescription = "Record audio",
                        isSelected = selectedMode == CreationMode.AUDIO,
                        isPrimary = true,
                        onClick = {
                            onModeSelected(CreationMode.AUDIO)
                            if (selectedMode == CreationMode.AUDIO) {
                                onPrimaryAction()
                            }
                        }
                    )

                    // Camera button
                    ModeButton(
                        icon = Icons.Default.CameraAlt,
                        contentDescription = "Take photo",
                        isSelected = selectedMode == CreationMode.PHOTO,
                        onClick = {
                            onModeSelected(CreationMode.PHOTO)
                            if (selectedMode == CreationMode.PHOTO) {
                                onPrimaryAction()
                            }
                        }
                    )

                    // Attachment button
                    ModeButton(
                        icon = Icons.Default.AttachFile,
                        contentDescription = "Add attachment",
                        isSelected = selectedMode == CreationMode.ATTACHMENT,
                        onClick = {
                            onModeSelected(CreationMode.ATTACHMENT)
                            if (selectedMode == CreationMode.ATTACHMENT) {
                                onPrimaryAction()
                            }
                        }
                    )

                    // Text button
                    ModeButton(
                        icon = Icons.Default.TextFields,
                        contentDescription = "Add text",
                        isSelected = selectedMode == CreationMode.TEXT,
                        onClick = {
                            onModeSelected(CreationMode.TEXT)
                            if (selectedMode == CreationMode.TEXT) {
                                onPrimaryAction()
                            }
                        }
                    )
                }
            }
        }

        // Collapsed FAB (shown when toolbar is collapsed)
        if (!isExpanded) {
            PrimaryFAB(
                mode = selectedMode,
                onClick = onToggleExpand,
                onLongClick = onPrimaryAction
            )
        }
    }
}

@Composable
private fun ModeButton(
    icon: ImageVector,
    contentDescription: String,
    isSelected: Boolean,
    isPrimary: Boolean = false,
    onClick: () -> Unit
) {
    val backgroundColor = when {
        isSelected && isPrimary -> GradestaGreen
        isSelected -> GradestaGreenLight
        else -> Color.Transparent
    }

    val iconColor = when {
        isSelected && isPrimary -> Color.White
        isSelected -> GradestaGreen
        else -> MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
    }

    IconButton(
        onClick = onClick,
        modifier = Modifier
            .size(if (isPrimary) 56.dp else 48.dp)
            .background(backgroundColor, CircleShape),
        colors = IconButtonDefaults.iconButtonColors(
            contentColor = iconColor
        )
    ) {
        Icon(
            imageVector = icon,
            contentDescription = contentDescription,
            modifier = Modifier.size(if (isPrimary) 28.dp else 24.dp)
        )
    }
}

@Composable
private fun PrimaryFAB(
    mode: CreationMode,
    onClick: () -> Unit,
    onLongClick: () -> Unit
) {
    val icon = when (mode) {
        CreationMode.AUDIO -> Icons.Default.Mic
        CreationMode.PHOTO -> Icons.Default.CameraAlt
        CreationMode.TEXT -> Icons.Default.TextFields
        CreationMode.ATTACHMENT -> Icons.Default.AttachFile
    }

    FloatingActionButton(
        onClick = onClick,
        containerColor = GradestaGreen,
        contentColor = Color.White,
        elevation = FloatingActionButtonDefaults.elevation(8.dp),
        shape = CircleShape,
        modifier = Modifier.size(64.dp)
    ) {
        Icon(
            imageVector = icon,
            contentDescription = "Create content",
            modifier = Modifier.size(32.dp)
        )
    }
}
