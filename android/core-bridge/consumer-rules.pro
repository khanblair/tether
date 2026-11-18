# JNA and the UniFFI generated bindings are loaded by reflection and by native code.
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
-keep class app.tether.core.generated.** { *; }
-dontwarn java.awt.*
