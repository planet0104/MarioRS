package com.mariogame.mario;

import android.util.Log;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;

/**
 * 手柄控制器类
 * 专门处理USB/蓝牙手柄输入，与遥控器逻辑完全分离
 * 
 * 支持的输入:
 * - 摇杆轴值 (左/右摇杆)
 * - DPAD方向键
 * - A/B/X/Y按钮
 * - 肩键 (LB/RB/LT/RT)
 * - 功能键 (SELECT/START)
 * 
 * 与RemoteController的区别:
 * - 不使用延迟释放
 * - 不自动开启加速模式
 * - 直接转发按键状态到Rust层
 */
public class GamepadController {
    private static final String TAG = "GamepadController";
    
    // 手柄轴ID常量 (与Rust joystick_android.rs保持一致)
    public static final int AXIS_X = 0;          // 左摇杆X
    public static final int AXIS_Y = 1;          // 左摇杆Y
    public static final int AXIS_Z = 2;          // 右摇杆X
    public static final int AXIS_RZ = 3;         // 右摇杆Y
    public static final int AXIS_LTRIGGER = 4;   // 左扳机
    public static final int AXIS_RTRIGGER = 5;   // 右扳机
    public static final int AXIS_HAT_X = 6;      // DPAD X
    public static final int AXIS_HAT_Y = 7;      // DPAD Y
    
    // 回调接口
    public interface GamepadCallback {
        // 检测到手柄输入时回调 (用于隐藏虚拟按钮)
        void onGamepadInputDetected();
    }
    
    // 手柄连接状态
    private boolean connected = false;
    
    // 回调
    private GamepadCallback callback;
    
    /**
     * 设置手柄回调
     */
    public void setCallback(GamepadCallback callback) {
        this.callback = callback;
    }
    
    /**
     * 检查事件源是否为手柄
     */
    public static boolean isGamepadSource(int source) {
        return (source & InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD ||
               (source & InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK;
    }
    
    /**
     * 检查按键是否为手柄专用按键 (不包括DPAD)
     */
    public static boolean isGamepadOnlyKey(int keyCode) {
        switch (keyCode) {
            // 主要按钮
            case KeyEvent.KEYCODE_BUTTON_A:
            case KeyEvent.KEYCODE_BUTTON_B:
            case KeyEvent.KEYCODE_BUTTON_X:
            case KeyEvent.KEYCODE_BUTTON_Y:
            // 肩键
            case KeyEvent.KEYCODE_BUTTON_L1:
            case KeyEvent.KEYCODE_BUTTON_R1:
            case KeyEvent.KEYCODE_BUTTON_L2:
            case KeyEvent.KEYCODE_BUTTON_R2:
            // 功能键
            case KeyEvent.KEYCODE_BUTTON_SELECT:
            case KeyEvent.KEYCODE_BUTTON_START:
            case KeyEvent.KEYCODE_BUTTON_MODE:
            // 摇杆按下
            case KeyEvent.KEYCODE_BUTTON_THUMBL:
            case KeyEvent.KEYCODE_BUTTON_THUMBR:
                return true;
            default:
                return false;
        }
    }
    
    /**
     * 处理手柄按键事件
     * 
     * @param event KeyEvent事件
     * @return true如果事件被处理
     */
    public boolean handleKeyEvent(KeyEvent event) {
        int keyCode = event.getKeyCode();
        int action = event.getAction();
        
        // 只处理按下和抬起事件
        if (action != KeyEvent.ACTION_DOWN && action != KeyEvent.ACTION_UP) {
            return false;
        }
        
        boolean pressed = (action == KeyEvent.ACTION_DOWN);
        
        // 标记手柄已连接
        if (!connected) {
            connected = true;
            Log.i(TAG, "手柄已连接");
            notifyGamepadDetected();
        }
        
        // 处理手柄按键
        switch (keyCode) {
            // DPAD方向键 - 来自手柄的DPAD
            case KeyEvent.KEYCODE_DPAD_UP:
            case KeyEvent.KEYCODE_DPAD_DOWN:
            case KeyEvent.KEYCODE_DPAD_LEFT:
            case KeyEvent.KEYCODE_DPAD_RIGHT:
                Log.d(TAG, "[Gamepad DPAD] keyCode=" + keyCode + ", pressed=" + pressed);
                MainActivity.nativeOnGamepadButton(keyCode, pressed);
                return true;
            
            // 主要按钮 A/B/X/Y
            case KeyEvent.KEYCODE_BUTTON_A:
            case KeyEvent.KEYCODE_BUTTON_B:
            case KeyEvent.KEYCODE_BUTTON_X:
            case KeyEvent.KEYCODE_BUTTON_Y:
                Log.d(TAG, "[Gamepad Button] keyCode=" + keyCode + ", pressed=" + pressed);
                MainActivity.nativeOnGamepadButton(keyCode, pressed);
                return true;
            
            // 肩键 LB/RB
            case KeyEvent.KEYCODE_BUTTON_L1:
            case KeyEvent.KEYCODE_BUTTON_R1:
                Log.d(TAG, "[Gamepad Shoulder] keyCode=" + keyCode + ", pressed=" + pressed);
                MainActivity.nativeOnGamepadButton(keyCode, pressed);
                return true;
            
            // 扳机 LT/RT (数字按钮形式)
            case KeyEvent.KEYCODE_BUTTON_L2:
            case KeyEvent.KEYCODE_BUTTON_R2:
                Log.d(TAG, "[Gamepad Trigger] keyCode=" + keyCode + ", pressed=" + pressed);
                MainActivity.nativeOnGamepadButton(keyCode, pressed);
                return true;
            
            // 功能键
            case KeyEvent.KEYCODE_BUTTON_SELECT:
            case KeyEvent.KEYCODE_BUTTON_START:
            case KeyEvent.KEYCODE_BUTTON_MODE:
                Log.d(TAG, "[Gamepad Function] keyCode=" + keyCode + ", pressed=" + pressed);
                MainActivity.nativeOnGamepadButton(keyCode, pressed);
                return true;
            
            // 摇杆按下
            case KeyEvent.KEYCODE_BUTTON_THUMBL:
            case KeyEvent.KEYCODE_BUTTON_THUMBR:
                Log.d(TAG, "[Gamepad Thumb] keyCode=" + keyCode + ", pressed=" + pressed);
                MainActivity.nativeOnGamepadButton(keyCode, pressed);
                return true;
            
            default:
                return false;
        }
    }
    
    /**
     * 处理手柄摇杆/扳机轴事件
     * 
     * @param event MotionEvent事件
     * @return true如果事件被处理
     */
    public boolean handleMotionEvent(MotionEvent event) {
        // 只处理来自手柄/摇杆的motion事件
        if (!isGamepadSource(event.getSource())) {
            return false;
        }
        
        // 标记手柄已连接
        if (!connected) {
            connected = true;
            Log.i(TAG, "手柄已连接 (通过摇杆)");
            notifyGamepadDetected();
        }
        
        // 读取左摇杆
        float leftX = event.getAxisValue(MotionEvent.AXIS_X);
        float leftY = event.getAxisValue(MotionEvent.AXIS_Y);
        
        // 读取右摇杆
        float rightX = event.getAxisValue(MotionEvent.AXIS_Z);
        float rightY = event.getAxisValue(MotionEvent.AXIS_RZ);
        
        // 读取扳机 (模拟轴)
        float leftTrigger = event.getAxisValue(MotionEvent.AXIS_LTRIGGER);
        float rightTrigger = event.getAxisValue(MotionEvent.AXIS_RTRIGGER);
        
        // 读取HAT轴 (某些手柄的DPAD)
        float hatX = event.getAxisValue(MotionEvent.AXIS_HAT_X);
        float hatY = event.getAxisValue(MotionEvent.AXIS_HAT_Y);
        
        // 发送轴数据到Rust
        MainActivity.nativeOnGamepadAxis(AXIS_X, leftX);
        MainActivity.nativeOnGamepadAxis(AXIS_Y, leftY);
        MainActivity.nativeOnGamepadAxis(AXIS_Z, rightX);
        MainActivity.nativeOnGamepadAxis(AXIS_RZ, rightY);
        MainActivity.nativeOnGamepadAxis(AXIS_LTRIGGER, leftTrigger);
        MainActivity.nativeOnGamepadAxis(AXIS_RTRIGGER, rightTrigger);
        MainActivity.nativeOnGamepadAxis(AXIS_HAT_X, hatX);
        MainActivity.nativeOnGamepadAxis(AXIS_HAT_Y, hatY);
        
        return true;
    }
    
    /**
     * 通知检测到手柄
     */
    private void notifyGamepadDetected() {
        if (callback != null) {
            callback.onGamepadInputDetected();
        }
    }
    
    /**
     * 检查手柄是否已连接
     */
    public boolean isConnected() {
        return connected;
    }
    
    /**
     * 重置手柄状态 (Activity暂停时调用)
     */
    public void reset() {
        // 手柄不需要特殊的释放逻辑
        // 状态由Rust层管理
        Log.i(TAG, "手柄状态已重置");
    }
}
