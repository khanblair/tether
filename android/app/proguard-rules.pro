# JNA and the UniFFI bindings are also kept by core-bridge/consumer-rules.pro.
# Keep rules for app code go here.
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
-keep class app.tether.core.generated.** { *; }
-dontwarn java.awt.*
