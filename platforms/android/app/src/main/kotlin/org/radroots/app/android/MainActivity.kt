package org.radroots.app.android

import android.os.Bundle
import com.google.androidgamesdk.GameActivity
import org.radroots.app.android.security.RadRootsAndroidSecurityBridge

class MainActivity : GameActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        RadRootsAndroidAppBridge.initialize(this)
        RadRootsAndroidSecurityBridge.initialize(this)
        super.onCreate(savedInstanceState)
    }
}
