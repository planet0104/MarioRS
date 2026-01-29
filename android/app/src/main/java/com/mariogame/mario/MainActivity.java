package com.mariogame.mario;

import android.os.Bundle;
import android.util.DisplayMetrics;
import android.util.Log;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;

import com.google.androidgamesdk.GameActivity;

import com.mariogame.R;

/**
 * 自定义 MainActivity
 * 继承 GameActivity, 使用模块化的控制器来处理输入
 * 
 * 输入处理架构 (手柄与遥控器完全分离):
 * - GamepadController: 处理USB/蓝牙手柄输入
 * - RemoteController: 处理TV遥控器/键盘按键
 * - VirtualController: 处理屏幕虚拟触摸按钮
 * 
 * 输入路由规则:
 * - SOURCE_GAMEPAD/SOURCE_JOYSTICK -> GamepadController
 * - 其他来源 (DPAD/键盘/遥控器) -> RemoteController
 * - 手柄专用按键 (BUTTON_A等) -> GamepadController
 * - 触摸事件 -> VirtualController
 */
public class MainActivity extends GameActivity {
    private static final String TAG = "MarioRS";
    
    // 屏幕尺寸
    private int screenWidth, screenHeight;
    
    // 控制器 (完全分离)
    private VirtualController virtualController;
    private RemoteController remoteController;
    private GamepadController gamepadController;
    
    // Native 方法声明 - 由 Rust 实现
    // 虚拟按钮事件 (屏幕触摸)
    public static native void nativeOnButtonEvent(int buttonId, boolean pressed);
    
    // 虚拟键盘按键事件
    public static native void nativeOnKeyEvent(int keyCode, boolean pressed);
    
    // 手柄专用JNI接口 (与遥控器分离)
    public static native void nativeOnGamepadButton(int keyCode, boolean pressed);
    public static native void nativeOnGamepadAxis(int axisId, float value);
    
    // 遥控器专用JNI接口 (与手柄分离)
    public static native void nativeOnRemoteKey(int keyCode, boolean pressed);
    
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        
        // 获取屏幕尺寸
        DisplayMetrics metrics = new DisplayMetrics();
        getWindowManager().getDefaultDisplay().getRealMetrics(metrics);
        screenWidth = metrics.widthPixels;
        screenHeight = metrics.heightPixels;
        
        Log.i(TAG, "Screen size: " + screenWidth + "x" + screenHeight);
        
        // 初始化控制器 (完全分离)
        virtualController = new VirtualController(this, screenWidth, screenHeight);
        remoteController = new RemoteController();
        gamepadController = new GamepadController();
        
        // 设置遥控器回调 (检测到遥控器时隐藏虚拟按钮)
        remoteController.setInputCallback(new RemoteController.InputCallback() {
            @Override
            public void onRemoteInputDetected() {
                hideVirtualButtons();
            }
        });
        
        // 设置手柄回调 (检测到手柄时隐藏虚拟按钮)
        gamepadController.setCallback(new GamepadController.GamepadCallback() {
            @Override
            public void onGamepadInputDetected() {
                hideVirtualButtons();
            }
        });
        
        // 延迟添加按钮层
        getWindow().getDecorView().post(this::addButtonOverlay);
    }
    
    /**
     * 隐藏虚拟按钮
     */
    private void hideVirtualButtons() {
        runOnUiThread(() -> {
            if (virtualController != null) {
                virtualController.hideGameButtons();
            }
        });
    }
    
    /**
     * 添加按钮覆盖层
     */
    private void addButtonOverlay() {
        ViewGroup contentView = findViewById(android.R.id.content);
        if (contentView == null) {
            Log.e(TAG, "Content view not found");
            return;
        }

        View existing = contentView.findViewById(R.id.button_layer);
        if (existing != null) {
            Log.i(TAG, "Button overlay already present, skip adding");
            return;
        }

        FrameLayout buttonLayer = virtualController.createButtonOverlay(contentView);
        contentView.addView(buttonLayer);

        Log.i(TAG, "Button overlay added");
    }
    
    /**
     * 检查事件源是否为手柄
     */
    private boolean isGamepadSource(int source) {
        return (source & InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD ||
               (source & InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK;
    }
    
    /**
     * 处理按键按下事件
     * 使用 onKeyDown/onKeyUp 代替 dispatchKeyEvent，因为 GameActivity 不调用 dispatchKeyEvent
     */
    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        return handleKeyEventInternal(keyCode, event) || super.onKeyDown(keyCode, event);
    }
    
    @Override
    public boolean onKeyUp(int keyCode, KeyEvent event) {
        return handleKeyEventInternal(keyCode, event) || super.onKeyUp(keyCode, event);
    }
    
    /**
     * 内部按键处理逻辑
     * 
     * 路由规则:
     * 1. 手柄专用按键 (BUTTON_A等) -> GamepadController
     * 2. 来自手柄的DPAD -> GamepadController  
     * 3. 来自遥控器/键盘的按键 -> RemoteController
     */
    private boolean handleKeyEventInternal(int keyCode, KeyEvent event) {
        int source = event.getSource();
        
        // 手柄专用按键 -> GamepadController (优先级最高)
        if (GamepadController.isGamepadOnlyKey(keyCode)) {
            if (gamepadController.handleKeyEvent(event)) {
                return true;
            }
        }
        
        // 来自手柄的DPAD按键 -> GamepadController
        if (isGamepadSource(source) && isDpadKey(keyCode)) {
            if (gamepadController.handleKeyEvent(event)) {
                return true;
            }
        }
        
        // 来自遥控器/键盘的按键 -> RemoteController
        if (remoteController.handleKeyEvent(event)) {
            return true;
        }
        
        return false;
    }
    
    /**
     * 检查是否为DPAD按键
     */
    private boolean isDpadKey(int keyCode) {
        switch (keyCode) {
            case KeyEvent.KEYCODE_DPAD_UP:
            case KeyEvent.KEYCODE_DPAD_DOWN:
            case KeyEvent.KEYCODE_DPAD_LEFT:
            case KeyEvent.KEYCODE_DPAD_RIGHT:
            case KeyEvent.KEYCODE_DPAD_CENTER:
                return true;
            default:
                return false;
        }
    }
    
    /**
     * 拦截手柄摇杆/扳机事件
     * 
     * 只有手柄会产生摇杆事件，直接转发给GamepadController
     */
    @Override
    public boolean onGenericMotionEvent(MotionEvent event) {
        // 手柄摇杆事件 -> GamepadController
        if (gamepadController.handleMotionEvent(event)) {
            return true;
        }
        
        return super.onGenericMotionEvent(event);
    }
    
    @Override
    protected void onPause() {
        super.onPause();
        
        // 释放所有控制器状态
        if (remoteController != null) {
            remoteController.releaseAllKeys();
        }
        if (gamepadController != null) {
            gamepadController.reset();
        }
    }
    
    @Override
    protected void onDestroy() {
        super.onDestroy();
        
        if (remoteController != null) {
            remoteController.releaseAllKeys();
        }
        if (gamepadController != null) {
            gamepadController.reset();
        }
    }
}
