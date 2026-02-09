package com.gradesta.android.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.gradesta.android.data.repository.ConnectionState
import com.gradesta.android.ui.theme.GradestaGreen
import com.gradesta.android.viewmodel.GraphViewModel

/**
 * Connection screen for entering server URL and landmark.
 */
@Composable
fun ConnectScreen(
    viewModel: GraphViewModel,
    onConnected: () -> Unit,
    modifier: Modifier = Modifier
) {
    val serverUrl by viewModel.serverUrl.collectAsState()
    val landmark by viewModel.landmark.collectAsState()
    val connectionState by viewModel.connectionState.collectAsState()
    val statusMessage by viewModel.statusMessage.collectAsState()

    val keyboardController = LocalSoftwareKeyboardController.current

    // Navigate to main screen when connected
    if (connectionState == ConnectionState.Connected) {
        onConnected()
    }

    Surface(
        modifier = modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.background
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            // Logo/Title
            Text(
                text = "Gradesta",
                style = MaterialTheme.typography.headlineLarge,
                color = GradestaGreen
            )

            Spacer(modifier = Modifier.height(48.dp))

            // Server URL input
            OutlinedTextField(
                value = serverUrl,
                onValueChange = { viewModel.setServerUrl(it) },
                label = { Text("Server URL") },
                placeholder = { Text("ws://localhost:8080") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
                keyboardOptions = KeyboardOptions(
                    keyboardType = KeyboardType.Uri,
                    imeAction = ImeAction.Next
                )
            )

            Spacer(modifier = Modifier.height(16.dp))

            // Landmark input
            OutlinedTextField(
                value = landmark,
                onValueChange = { viewModel.setLandmark(it) },
                label = { Text("Landmark") },
                placeholder = { Text("/") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
                keyboardOptions = KeyboardOptions(
                    keyboardType = KeyboardType.Text,
                    imeAction = ImeAction.Done
                ),
                keyboardActions = KeyboardActions(
                    onDone = {
                        keyboardController?.hide()
                        viewModel.connect()
                    }
                )
            )

            Spacer(modifier = Modifier.height(32.dp))

            // Connect button
            Button(
                onClick = {
                    keyboardController?.hide()
                    viewModel.connect()
                },
                enabled = connectionState != ConnectionState.Connecting,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(56.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = GradestaGreen
                )
            ) {
                if (connectionState == ConnectionState.Connecting) {
                    CircularProgressIndicator(
                        color = MaterialTheme.colorScheme.onPrimary,
                        modifier = Modifier.padding(end = 8.dp)
                    )
                    Text("Connecting...")
                } else {
                    Text("Connect")
                }
            }

            // Status message
            statusMessage?.let { message ->
                Spacer(modifier = Modifier.height(16.dp))
                Text(
                    text = message,
                    style = MaterialTheme.typography.bodyMedium,
                    color = if (message.startsWith("Error")) {
                        MaterialTheme.colorScheme.error
                    } else {
                        MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                    }
                )
            }
        }
    }
}
