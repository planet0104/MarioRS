# MarioRS ProGuard 规则
# 纯 Native Activity 应用，无需特殊规则
# 保留 native 方法
-keepclasseswithmembernames class * {
    native <methods>;
}
