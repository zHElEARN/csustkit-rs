package com.zhelearn.csustkit;

import android.content.Context;
import java.util.Objects;

public final class CsustKitAndroid {
    static {
        System.loadLibrary("csustkit");
    }

    private CsustKitAndroid() {}

    /**
     * Initializes Android certificate verification before creating a session.
     * This can be called more than once and retains only the application context.
     */
    public static void initialize(Context context) {
        Context application = Objects.requireNonNull(
            context.getApplicationContext(), "An application context is required");
        initializeTls(application);
    }

    private static native void initializeTls(Context context);
}
