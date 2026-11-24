# Preserve methods resolved by JNI or JNA and fields read by JNA.
-keepclasseswithmembers class com.zhelearn.csustkit.** {
    native <methods>;
}
-keep class com.zhelearn.csustkit.** extends com.sun.jna.Structure { *; }
-keep interface com.zhelearn.csustkit.** extends com.sun.jna.Callback { *; }
-keep class com.sun.jna.** { *; }
-dontwarn java.awt.**

# The Rust TLS verifier looks up these JVM classes by name.
-keep,includedescriptorclasses class org.rustls.platformverifier.** { *; }
