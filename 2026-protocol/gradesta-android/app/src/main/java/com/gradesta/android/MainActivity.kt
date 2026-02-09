package com.gradesta.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Surface
import androidx.compose.ui.Modifier
import com.gradesta.android.ui.navigation.GradestaNavGraph
import com.gradesta.android.ui.theme.GradestaTheme
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            GradestaTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    GradestaNavGraph()
                }
            }
        }
    }
}
