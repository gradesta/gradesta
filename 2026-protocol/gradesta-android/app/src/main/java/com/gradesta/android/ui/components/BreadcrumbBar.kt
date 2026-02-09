package com.gradesta.android.ui.components

import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ChevronRight
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.gradesta.android.ui.theme.GradestaGreen

/**
 * Top breadcrumb navigation bar.
 */
@Composable
fun BreadcrumbBar(
    path: String?,
    onMenuClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Surface(
        modifier = modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surface,
        shadowElevation = 2.dp
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 8.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            // Breadcrumb path
            BreadcrumbPath(
                path = path ?: "/",
                modifier = Modifier.weight(1f)
            )

            // Menu button
            IconButton(onClick = onMenuClick) {
                Icon(
                    imageVector = Icons.Default.Menu,
                    contentDescription = "Menu"
                )
            }
        }
    }
}

@Composable
private fun BreadcrumbPath(
    path: String,
    modifier: Modifier = Modifier
) {
    val segments = path.split("/").filter { it.isNotEmpty() }

    Row(
        modifier = modifier.horizontalScroll(rememberScrollState()),
        verticalAlignment = Alignment.CenterVertically
    ) {
        if (segments.isEmpty()) {
            Text(
                text = "Home",
                style = MaterialTheme.typography.titleMedium,
                color = GradestaGreen
            )
        } else {
            segments.forEachIndexed { index, segment ->
                if (index > 0) {
                    Icon(
                        imageVector = Icons.Default.ChevronRight,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f),
                        modifier = Modifier.padding(horizontal = 4.dp)
                    )
                }

                val isLast = index == segments.lastIndex
                Text(
                    text = segment,
                    style = MaterialTheme.typography.titleMedium,
                    color = if (isLast) GradestaGreen else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.clickable(enabled = !isLast) {
                        // TODO: Navigate to parent path
                    }
                )
            }
        }
    }
}
