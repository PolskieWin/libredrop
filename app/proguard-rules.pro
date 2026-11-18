# LibreDrop ProGuard Rules
-keep class org.libredrop.engine.NativeEngine { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}
-keep class org.libredrop.model.** { *; }
-keepclassmembers class org.libredrop.model.** { *; }
-keepattributes *Annotation*
-dontwarn java.lang.invoke.**
