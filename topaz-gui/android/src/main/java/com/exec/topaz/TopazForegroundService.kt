package com.exec.topaz

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.app.ServiceInfo
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat

/**
 * Foreground Service to keep the Topaz decompilation server running in the background.
 * This service is started by the Rust code via startForegroundService().
 * It calls startForeground() with a persistent notification, which is REQUIRED
 * on Android 8+ (API 26+) for any long-running background work.
 */
class TopazForegroundService : Service() {

    companion object {
        const val NOTIFICATION_ID = 1001
        const val CHANNEL_ID = "topaz_server_channel"
        const val EXTRA_PORT = "server_port"
        const val ACTION_STOP = "com.exec.topaz.ACTION_STOP_SERVER"
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            stopForeground(true)
            stopSelf()
            return START_NOT_STICKY
        }

        val port = intent?.getIntExtra(EXTRA_PORT, 3000) ?: 3000
        val notification = buildNotification(port)

        // Start as foreground service - this is the critical call that prevents
        // the process from being killed when the app is backgrounded
        startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)

        return START_STICKY
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Topaz Decompilation Server",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Local decompilation server status"
                setSound(null, null) // Silent
                enableVibration(false)
            }
            val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(port: Int): Notification {
        // Intent to open the app when notification is tapped
        val openAppIntent = packageManager.getLaunchIntentForPackage("com.exec.topaz")
        val contentIntent = if (openAppIntent != null) {
            PendingIntent.getActivity(
                this,
                0,
                openAppIntent,
                PendingIntent.FLAG_IMMUTABLE
            )
        } else {
            null
        }

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Topaz Server Running")
            .setContentText("Listening on 0.0.0.0:$port")
            .setSmallIcon(android.R.drawable.stat_sys_download) // Fallback icon
            .setOngoing(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setOnlyAlertOnce(true)
            .setContentIntent(contentIntent)
            .build()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        stopForeground(true)
        super.onDestroy()
    }
}