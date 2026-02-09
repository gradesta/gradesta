package com.gradesta.android.ui.screens

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import com.gradesta.android.data.model.Direction
import com.gradesta.android.data.model.Vertex
import com.gradesta.android.data.repository.ConnectionState
import com.gradesta.android.ui.components.BreadcrumbBar
import com.gradesta.android.ui.components.CreationToolbar
import com.gradesta.android.ui.components.VertexCard
import com.gradesta.android.viewmodel.GraphViewModel
import kotlinx.coroutines.launch
import kotlin.math.abs

/**
 * Main graph screen with swipe navigation.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalFoundationApi::class)
@Composable
fun MainScreen(
    viewModel: GraphViewModel,
    onDisconnected: () -> Unit,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()
    val creationMode by viewModel.creationMode.collectAsState()
    val isToolbarExpanded by viewModel.isToolbarExpanded.collectAsState()
    val connectionState by viewModel.connectionState.collectAsState()

    val drawerState = rememberDrawerState(DrawerValue.Closed)
    val scope = rememberCoroutineScope()

    // Navigate back to connect screen if disconnected
    LaunchedEffect(connectionState) {
        if (connectionState == ConnectionState.Disconnected) {
            onDisconnected()
        }
    }

    ModalNavigationDrawer(
        drawerState = drawerState,
        drawerContent = {
            ModalDrawerSheet {
                DrawerContent(
                    onDisconnect = {
                        viewModel.disconnect()
                    },
                    onCloseDrawer = {
                        scope.launch { drawerState.close() }
                    }
                )
            }
        }
    ) {
        Scaffold(
            topBar = {
                BreadcrumbBar(
                    path = uiState.graphState.contextUri,
                    onMenuClick = {
                        scope.launch { drawerState.open() }
                    }
                )
            },
            bottomBar = {
                CreationToolbar(
                    selectedMode = creationMode,
                    isExpanded = isToolbarExpanded,
                    onModeSelected = { viewModel.setCreationMode(it) },
                    onPrimaryAction = {
                        // TODO: Handle primary action based on mode
                    },
                    onToggleExpand = { viewModel.toggleToolbar() }
                )
            }
        ) { paddingValues ->
            SwipeableCardList(
                viewModel = viewModel,
                uiState = uiState,
                modifier = Modifier.padding(paddingValues)
            )
        }
    }
}

@Composable
private fun SwipeableCardList(
    viewModel: GraphViewModel,
    uiState: com.gradesta.android.viewmodel.MainUiState,
    modifier: Modifier = Modifier
) {
    val listState = rememberLazyListState()
    var horizontalDrag by remember { mutableFloatStateOf(0f) }
    var isNavigating by remember { mutableStateOf(false) }

    val swipeThreshold = 100f

    // Build list of vertices to display (North/South navigation)
    val verticalList = buildVerticalList(uiState)

    Box(
        modifier = modifier
            .fillMaxSize()
            .pointerInput(Unit) {
                detectHorizontalDragGestures(
                    onDragStart = {
                        horizontalDrag = 0f
                        isNavigating = false
                    },
                    onDragEnd = {
                        if (!isNavigating && abs(horizontalDrag) > swipeThreshold) {
                            isNavigating = true
                            if (horizontalDrag > 0) {
                                viewModel.navigateWest()
                            } else {
                                viewModel.navigateEast()
                            }
                        }
                        horizontalDrag = 0f
                    },
                    onHorizontalDrag = { _, dragAmount ->
                        horizontalDrag += dragAmount
                    }
                )
            }
    ) {
        if (verticalList.isEmpty()) {
            // Empty state
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = "Waiting for data...",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
                )
            }
        } else {
            LazyColumn(
                state = listState,
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
                modifier = Modifier.fillMaxSize()
            ) {
                items(
                    items = verticalList,
                    key = { it.id.toLong() }
                ) { vertex ->
                    val isSelected = vertex.id == uiState.currentVertex?.id

                    VertexCard(
                        vertex = vertex,
                        isSelected = isSelected,
                        modifier = Modifier
                            .fillMaxWidth(),
                        onClick = {
                            if (!isSelected) {
                                viewModel.navigateTo(vertex.id)

                                // Determine direction based on position in list
                                val currentIndex = verticalList.indexOfFirst {
                                    it.id == uiState.currentVertex?.id
                                }
                                val targetIndex = verticalList.indexOf(vertex)
                                if (targetIndex < currentIndex) {
                                    viewModel.setLastDirection(Direction.NORTH)
                                } else {
                                    viewModel.setLastDirection(Direction.SOUTH)
                                }
                            }
                        }
                    )
                }
            }

            // Auto-scroll to current vertex when it changes
            LaunchedEffect(uiState.currentVertex?.id) {
                val currentIndex = verticalList.indexOfFirst {
                    it.id == uiState.currentVertex?.id
                }
                if (currentIndex >= 0) {
                    listState.animateScrollToItem(currentIndex)
                }
            }
        }

        // Neighbor hints on sides
        NeighborHints(
            westNeighbor = uiState.neighbors.west,
            eastNeighbor = uiState.neighbors.east,
            modifier = Modifier.fillMaxSize()
        )
    }
}

/**
 * Build the vertical list of vertices (current + north/south neighbors).
 */
private fun buildVerticalList(uiState: com.gradesta.android.viewmodel.MainUiState): List<Vertex> {
    val result = mutableListOf<Vertex>()
    val current = uiState.currentVertex ?: return result
    val graph = uiState.graphState

    // Add north neighbors going up
    val northIds = mutableListOf<Vertex>()
    var northId = current.northNeighbor.toULong()
    while (northId != 0UL && northIds.size < 5) {
        graph.getVertex(northId)?.let {
            northIds.add(0, it)
            northId = it.northNeighbor.toULong()
        } ?: break
    }
    result.addAll(northIds)

    // Add current
    result.add(current)

    // Add south neighbors going down
    var southId = current.southNeighbor.toULong()
    var count = 0
    while (southId != 0UL && count < 5) {
        graph.getVertex(southId)?.let {
            result.add(it)
            southId = it.southNeighbor.toULong()
            count++
        } ?: break
    }

    return result
}

@Composable
private fun NeighborHints(
    westNeighbor: Vertex?,
    eastNeighbor: Vertex?,
    modifier: Modifier = Modifier
) {
    Box(modifier = modifier) {
        // West hint
        westNeighbor?.let {
            Box(
                modifier = Modifier
                    .align(Alignment.CenterStart)
                    .padding(start = 4.dp)
            ) {
                Text(
                    text = "◀",
                    style = MaterialTheme.typography.headlineSmall,
                    color = MaterialTheme.colorScheme.primary.copy(alpha = 0.5f)
                )
            }
        }

        // East hint
        eastNeighbor?.let {
            Box(
                modifier = Modifier
                    .align(Alignment.CenterEnd)
                    .padding(end = 4.dp)
            ) {
                Text(
                    text = "▶",
                    style = MaterialTheme.typography.headlineSmall,
                    color = MaterialTheme.colorScheme.primary.copy(alpha = 0.5f)
                )
            }
        }
    }
}

@Composable
private fun DrawerContent(
    onDisconnect: () -> Unit,
    onCloseDrawer: () -> Unit
) {
    Text(
        text = "Gradesta",
        style = MaterialTheme.typography.headlineMedium,
        modifier = Modifier.padding(16.dp)
    )

    androidx.compose.material3.NavigationDrawerItem(
        label = { Text("Settings") },
        selected = false,
        onClick = {
            // TODO: Navigate to settings
            onCloseDrawer()
        },
        modifier = Modifier.padding(horizontal = 12.dp)
    )

    androidx.compose.material3.NavigationDrawerItem(
        label = { Text("Disconnect") },
        selected = false,
        onClick = {
            onDisconnect()
            onCloseDrawer()
        },
        modifier = Modifier.padding(horizontal = 12.dp)
    )
}
