package com.mariogame.mario;

import android.util.Log;
import android.view.KeyEvent;

/**
 * 遥控器控制器类
 * 专门处理TV遥控器和键盘按键，与手柄逻辑完全分离
 * 
 * 遥控器检测说明:
 * - 只有按下 DPAD_CENTER (OK键) 才确认是真正的TV遥控器
 * - DPAD_CENTER 在PC键盘上不存在，是遥控器特有的按键
 * - 键盘用户会使用 Enter 键确认，不触发遥控器模式
 * - 只有真正的遥控器才会启用空中慢动作模式
 * 
 * 支持6个按键:
 * - 上键: 菜单向上/游戏中跳跃
 * - 下键: 菜单向下/游戏中向下(钻管道)/发射子弹
 * - 左键: 左行走
 * - 右键: 右行走
 * - OK键: 菜单确认/游戏中跳跃
 * - 返回键: ESC
 * 
 * 遥控器特性 (与手柄不同):
 * - 自动加速: 检测到遥控器时自动开启加速模式
 * - 空中慢动作: 跳跃后空中停留时间延长，便于单键操作时左右移动
 *   (物理调整在Rust层实现，Java层只负责检测和传递遥控器模式)
 * 
 * 注意: 此控制器不处理手柄输入，手柄由GamepadController处理
 */
public class RemoteController {
    private static final String TAG = "RemoteController";
    
    // 回调接口
    public interface InputCallback {
        // 检测到物理输入时回调 (用于隐藏虚拟按钮)
        // 注意: 任何物理输入(键盘/遥控器)都会触发，不只是遥控器
        void onRemoteInputDetected();
    }
    
    // 当前按下的方向键状态
    private boolean leftPressed = false;
    private boolean rightPressed = false;
    private boolean upPressed = false;
    private boolean downPressed = false;
    
    // 加速模式状态 (遥控器自动开启)
    private boolean accelerateMode = false;
    private boolean accelerateSent = false;
    
    // 是否检测到真正的TV遥控器 (非键盘)
    // 只有按下 DPAD_CENTER (OK键) 才确认是遥控器，因为这个键在PC键盘上不存在
    private boolean remoteDetected = false;
    
    // 是否已经隐藏虚拟按钮 (任何物理输入都会触发)
    private boolean virtualButtonsHidden = false;
    
    // 输入回调
    private InputCallback inputCallback;
    
    /**
     * 设置输入回调
     */
    public void setInputCallback(InputCallback callback) {
        this.inputCallback = callback;
    }
    
    /**
     * 处理遥控器按键事件
     * 
     * 注意: 只处理遥控器/键盘按键，手柄按键由GamepadController处理
     * 
     * @param event KeyEvent事件
     * @return true如果事件被处理, false交给其他处理器
     */
    public boolean handleKeyEvent(KeyEvent event) {
        int keyCode = event.getKeyCode();
        int action = event.getAction();
        
        // 只处理按下和抬起事件
        if (action != KeyEvent.ACTION_DOWN && action != KeyEvent.ACTION_UP) {
            return false;
        }
        
        boolean pressed = (action == KeyEvent.ACTION_DOWN);
        
        // 忽略重复按键事件 (长按时会产生)
        if (pressed && event.getRepeatCount() > 0) {
            return true;
        }
        
        // 音量键不拦截
        if (keyCode == KeyEvent.KEYCODE_VOLUME_UP || keyCode == KeyEvent.KEYCODE_VOLUME_DOWN) {
            return false;
        }
        
        // 检测到物理输入（键盘/遥控器），隐藏虚拟按钮
        if (pressed && !virtualButtonsHidden) {
            virtualButtonsHidden = true;
            Log.i(TAG, "[Remote] 检测到物理输入，隐藏虚拟按钮");
            notifyRemoteDetected();
        }
        
        // 处理遥控器核心按键
        switch (keyCode) {
            case KeyEvent.KEYCODE_DPAD_UP:
                handleUpKey(pressed);
                return true;
            
            case KeyEvent.KEYCODE_DPAD_DOWN:
                handleDownKey(pressed);
                return true;
            
            case KeyEvent.KEYCODE_DPAD_LEFT:
                handleLeftKey(pressed);
                return true;
            
            case KeyEvent.KEYCODE_DPAD_RIGHT:
                handleRightKey(pressed);
                return true;
            
            case KeyEvent.KEYCODE_DPAD_CENTER:
                // DPAD_CENTER (OK键) 是遥控器特有的，PC键盘上没有这个键
                // 只有按下这个键才确认是真正的TV遥控器
                if (pressed && !remoteDetected) {
                    Log.i(TAG, "[Remote] 检测到 DPAD_CENTER (OK键)，确认是TV遥控器");
                    setAsRealRemote();
                }
                handleOkKey(pressed);
                return true;
            
            case KeyEvent.KEYCODE_ENTER:
                // Enter键是键盘按键，不触发遥控器模式
                handleOkKey(pressed);
                return true;
            
            case KeyEvent.KEYCODE_BACK:
                handleBackKey(pressed);
                return true;
            
            default:
                // 其他按键不处理
                return false;
        }
    }
    
    /**
     * 设置为真正的TV遥控器模式
     * 只有按下 DPAD_CENTER (OK键) 才会调用此方法
     * 
     * 注意: 隐藏虚拟按钮已在 handleKeyEvent 中统一处理，这里不需要再调用
     */
    private void setAsRealRemote() {
        remoteDetected = true;
        Log.i(TAG, "[Remote 遥控器!!] 启用TV遥控器模式 (空中慢动作)");
        
        // 遥控器自动开启加速模式
        if (!accelerateMode) {
            accelerateMode = true;
            if (!accelerateSent) {
                accelerateSent = true;
                Log.i(TAG, "[Remote 遥控器!!] 自动加速模式开启");
                // 发送加速键 (使用遥控器专用JNI接口)
                MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_BUTTON_B, true);
            }
        }
    }
    
    /**
     * 通知检测到遥控器
     */
    private void notifyRemoteDetected() {
        if (inputCallback != null) {
            inputCallback.onRemoteInputDetected();
        }
    }
    
    // ========================================================================
    // 按键处理方法
    // 使用 nativeOnRemoteKey 发送原始 Android KeyCode 到 Rust 端
    // Rust 端的 joystick_android_tv.rs 模块会处理这些按键
    // ========================================================================
    
    /**
     * 获取日志前缀，区分遥控器和键盘
     */
    private String getLogPrefix() {
        return remoteDetected ? "[Remote 遥控器]" : "[DPAD 键盘]";
    }
    
    /**
     * 处理上键
     */
    private void handleUpKey(boolean pressed) {
        upPressed = pressed;
        Log.d(TAG, getLogPrefix() + " UP pressed=" + pressed);
        MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_UP, pressed);
    }
    
    /**
     * 处理下键
     */
    private void handleDownKey(boolean pressed) {
        downPressed = pressed;
        Log.d(TAG, getLogPrefix() + " DOWN pressed=" + pressed);
        MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_DOWN, pressed);
    }
    
    /**
     * 处理左键 (立即响应，空中慢动作在Rust层实现)
     */
    private void handleLeftKey(boolean pressed) {
        if (pressed) {
            if (!leftPressed) {
                leftPressed = true;
                Log.d(TAG, getLogPrefix() + " LEFT pressed");
                MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_LEFT, true);
            }
            
            // 按左键时立即取消右键
            if (rightPressed) {
                rightPressed = false;
                MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_RIGHT, false);
                Log.d(TAG, getLogPrefix() + " RIGHT cancelled by LEFT");
            }
        } else {
            // 立即释放，不再延迟
            if (leftPressed) {
                leftPressed = false;
                Log.d(TAG, getLogPrefix() + " LEFT released");
                MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_LEFT, false);
            }
        }
    }
    
    /**
     * 处理右键 (立即响应，空中慢动作在Rust层实现)
     */
    private void handleRightKey(boolean pressed) {
        if (pressed) {
            if (!rightPressed) {
                rightPressed = true;
                Log.d(TAG, getLogPrefix() + " RIGHT pressed");
                MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_RIGHT, true);
            }
            
            // 按右键时立即取消左键
            if (leftPressed) {
                leftPressed = false;
                MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_LEFT, false);
                Log.d(TAG, getLogPrefix() + " LEFT cancelled by RIGHT");
            }
        } else {
            // 立即释放，不再延迟
            if (rightPressed) {
                rightPressed = false;
                Log.d(TAG, getLogPrefix() + " RIGHT released");
                MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_RIGHT, false);
            }
        }
    }
    
    /**
     * 处理OK键
     */
    private void handleOkKey(boolean pressed) {
        Log.d(TAG, getLogPrefix() + " OK pressed=" + pressed);
        MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_CENTER, pressed);
    }
    
    /**
     * 处理返回键
     */
    private void handleBackKey(boolean pressed) {
        Log.d(TAG, getLogPrefix() + " BACK pressed=" + pressed);
        MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_BACK, pressed);
    }
    
    /**
     * 释放所有方向键
     */
    private void releaseDirectionKeys() {
        if (leftPressed) {
            leftPressed = false;
            MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_LEFT, false);
        }
        if (rightPressed) {
            rightPressed = false;
            MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_RIGHT, false);
        }
    }
    
    // ========================================================================
    // 公共方法
    // ========================================================================
    
    /**
     * 检查是否已检测到遥控器
     */
    public boolean isRemoteDetected() {
        return remoteDetected;
    }
    
    /**
     * 检查加速模式是否开启
     */
    public boolean isAccelerateModeOn() {
        return accelerateMode;
    }
    
    /**
     * 释放所有按键 (Activity暂停时调用)
     */
    public void releaseAllKeys() {
        releaseDirectionKeys();
        
        if (upPressed) {
            upPressed = false;
            MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_UP, false);
        }
        if (downPressed) {
            downPressed = false;
            MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_DPAD_DOWN, false);
        }
        
        // 重置加速状态
        if (accelerateSent) {
            accelerateSent = false;
            MainActivity.nativeOnRemoteKey(KeyEvent.KEYCODE_BUTTON_B, false);
        }
        
        Log.i(TAG, getLogPrefix() + " 所有按键已释放");
    }
}
