package com.exec.topaz;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ServiceInfo;
import android.os.Build;
import android.os.IBinder;
import android.os.PowerManager;

/**
 * A genuine foreground service. This — not Notification.Builder#setOngoing(true) on its own —
 * is what makes Android refuse the swipe-to-dismiss gesture on the notification. A plain
 * NotificationManager.notify() with setOngoing(true) and no service behind it is treated as
 * dismissable on stock Android 8+, and Samsung's One UI shade is particularly aggressive about
 * letting the user swipe those away.
 *
 * Running in its own process (process = ":service" in Cargo.toml) also means the HTTP server
 * this service now owns (via JNI, see service_entry.rs) survives the UI process dying and
 * restarting — which is what handles the winit one-EventLoop-per-process limit on reopen.
 * Two things are asked of this service via Intent extras on
 * startService()/startForegroundService():
 *   - "cmd" = "start_server" | "stop_server": start/stop the actual tokio HTTP server.
 *   - no "cmd" at all: just the Persistent-mode keepalive ping, with optional "text".
 */
public class KeepAliveService extends Service {
    private static final String CHANNEL_ID = "topaz_keepalive";
    private static final int NOTIF_ID = 9001;
    public static final String EXTRA_TEXT = "text";
    public static final String EXTRA_CMD = "cmd";
    public static final String EXTRA_PORT = "port";
    public static final String EXTRA_LUAU = "luau";
    public static final String EXTRA_LUA51 = "lua51";
    public static final String EXTRA_ENCODE_KEY = "encode_key";
    public static final String EXTRA_FILES_DIR = "files_dir";

    private PowerManager.WakeLock wakeLock;

    static {
        System.loadLibrary("topaz_gui");
    }

    private static native void nativeStartServer(
            int port, boolean luau, boolean lua51, int encodeKey, String filesDir);

    private static native void nativeStopServer();

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        String text = "Tap to open — Topaz is running in the background.";
        String cmd = null;

        if (intent != null) {
            if (intent.hasExtra(EXTRA_TEXT)) {
                String extra = intent.getStringExtra(EXTRA_TEXT);
                if (extra != null && !extra.isEmpty()) {
                    text = extra;
                }
            }
            cmd = intent.getStringExtra(EXTRA_CMD);
        }

        if ("start_server".equals(cmd)) {
            int port = intent.getIntExtra(EXTRA_PORT, 3000);
            boolean luau = intent.getBooleanExtra(EXTRA_LUAU, true);
            boolean lua51 = intent.getBooleanExtra(EXTRA_LUA51, false);
            int encodeKey = intent.getIntExtra(EXTRA_ENCODE_KEY, 0);
            String filesDir = intent.getStringExtra(EXTRA_FILES_DIR);

            acquireWakeLock();
            text = "Serving on port " + port + "…";
            nativeStartServer(port, luau, lua51, encodeKey, filesDir);
        } else if ("stop_server".equals(cmd)) {
            nativeStopServer();
            releaseWakeLock();
            text = "Tap to open — Topaz is running in the background.";
        }

        showForeground(text);
        // START_STICKY: if the system kills the process under memory pressure it will try to
        // recreate the service (with a null Intent) rather than leaving it dead. Note a null
        // Intent on restart means we can't re-issue "start_server" automatically — the server
        // has to be explicitly restarted from the UI in that case.
        return START_STICKY;
    }

    private void acquireWakeLock() {
        if (wakeLock != null && wakeLock.isHeld()) {
            return;
        }
        PowerManager pm = (PowerManager) getSystemService(Context.POWER_SERVICE);
        if (pm != null) {
            wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "topaz:server");
            wakeLock.acquire();
        }
    }

    private void releaseWakeLock() {
        if (wakeLock != null && wakeLock.isHeld()) {
            wakeLock.release();
        }
        wakeLock = null;
    }

    private void showForeground(String text) {
        NotificationManager nm = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && nm != null) {
            NotificationChannel channel = nm.getNotificationChannel(CHANNEL_ID);
            if (channel == null) {
                channel = new NotificationChannel(
                        CHANNEL_ID, "Topaz Keep-Alive", NotificationManager.IMPORTANCE_LOW);
                channel.setShowBadge(false);
                nm.createNotificationChannel(channel);
            }
        }

        PendingIntent contentIntent = null;
        Intent launchIntent = getPackageManager().getLaunchIntentForPackage(getPackageName());
        if (launchIntent != null) {
            // REORDER_TO_FRONT can revive a NativeActivity whose window/event
            // loop was disposed when its task was dismissed.  That is the
            // splash/logo hang: Android waits for that stale activity to draw.
            // Start a clean task instead, reusing only an already-top instance.
            launchIntent.setFlags(
                    Intent.FLAG_ACTIVITY_NEW_TASK
                            | Intent.FLAG_ACTIVITY_CLEAR_TOP
                            | Intent.FLAG_ACTIVITY_SINGLE_TOP);
            int piFlags = PendingIntent.FLAG_UPDATE_CURRENT;
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                piFlags |= PendingIntent.FLAG_IMMUTABLE;
            }
            contentIntent = PendingIntent.getActivity(this, 0, launchIntent, piFlags);
        }

        Notification.Builder builder = (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O)
                ? new Notification.Builder(this, CHANNEL_ID)
                : new Notification.Builder(this);

        builder.setContentTitle("Topaz")
                .setContentText(text)
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setOngoing(true)
                .setAutoCancel(false);
        if (contentIntent != null) {
            builder.setContentIntent(contentIntent);
        }

        Notification notification = builder.build();

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIF_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC);
        } else {
            startForeground(NOTIF_ID, notification);
        }
    }

    @Override
    public void onTaskRemoved(Intent rootIntent) {
        super.onTaskRemoved(rootIntent);
    }

    @Override
    public void onDestroy() {
        nativeStopServer();
        releaseWakeLock();
        stopForeground(true);
        super.onDestroy();
    }
}
