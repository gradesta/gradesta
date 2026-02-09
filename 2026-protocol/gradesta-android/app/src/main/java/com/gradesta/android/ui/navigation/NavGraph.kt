package com.gradesta.android.ui.navigation

import androidx.compose.runtime.Composable
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.gradesta.android.ui.screens.ConnectScreen
import com.gradesta.android.ui.screens.MainScreen
import com.gradesta.android.viewmodel.GraphViewModel

/**
 * Navigation routes.
 */
object Routes {
    const val CONNECT = "connect"
    const val MAIN = "main"
}

/**
 * Main navigation graph.
 */
@Composable
fun GradestaNavGraph(
    navController: NavHostController = rememberNavController(),
    startDestination: String = Routes.CONNECT
) {
    // Shared ViewModel across screens
    val viewModel: GraphViewModel = hiltViewModel()

    NavHost(
        navController = navController,
        startDestination = startDestination
    ) {
        composable(Routes.CONNECT) {
            ConnectScreen(
                viewModel = viewModel,
                onConnected = {
                    navController.navigate(Routes.MAIN) {
                        popUpTo(Routes.CONNECT) { inclusive = true }
                    }
                }
            )
        }

        composable(Routes.MAIN) {
            MainScreen(
                viewModel = viewModel,
                onDisconnected = {
                    navController.navigate(Routes.CONNECT) {
                        popUpTo(Routes.MAIN) { inclusive = true }
                    }
                }
            )
        }
    }
}
