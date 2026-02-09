package com.gradesta.android.di

import com.gradesta.android.data.network.WebSocketClient
import com.gradesta.android.data.repository.GraphRepository
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object AppModule {

    @Provides
    @Singleton
    fun provideWebSocketClient(): WebSocketClient {
        return WebSocketClient()
    }

    @Provides
    @Singleton
    fun provideGraphRepository(
        webSocketClient: WebSocketClient
    ): GraphRepository {
        return GraphRepository(webSocketClient)
    }
}
