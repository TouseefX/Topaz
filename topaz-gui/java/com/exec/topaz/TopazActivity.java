package com.exec.topaz;

import android.app.NativeActivity;
import android.content.Intent;
import android.os.Bundle;
import android.util.Log;

/**
 * Custom activity that extends NativeActivity.
 *
 * This is required when using a foreground service that keeps the process
 * alive after the user swipes the task away.
 *
 * Using the raw "android.app.NativeActivity" directly often leads to
 * splash screen hangs because Android tries to reuse a stale activity
 * instance when the service process is still running.
 *
 * By having our own subclass we can:
 *   - Force a clean onCreate path
 *   - Avoid singleTop / singleTask reuse issues
 *   - Have a proper place to handle future onNewIntent etc.
 */
public class TopazActivity extends NativeActivity {
    private static final String TAG = "TopazActivity";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        Log.i(TAG, "onCreate called");
        super.onCreate(savedInstanceState);
    }

    @Override
    protected void onNewIntent(Intent intent) {
        Log.i(TAG, "onNewIntent called");
        super.onNewIntent(intent);
        setIntent(intent);
    }
    
    @Override
    public void onBackPressed() {
        Log.i(TAG, "onBackPressed - clearing activity instance safely");
        super.onBackPressed();
        this.finish();
        
        android.os.Process.killProcess(android.os.Process.myPid());
    }
    
    @Override
    protected void onStop() {
        Log.i(TAG, "onStop - closing activity window to protect Rust runtime");
        super.onStop();
        if (!isFinishing()) {
            this.finish();
            
            android.os.Process.killProcess(android.os.Process.myPid());
        }
    }

    @Override
    protected void onDestroy() {
        Log.i(TAG, "onDestroy called");
        super.onDestroy();
    }
}
