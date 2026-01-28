package com.mariogame.mario;

import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.view.KeyEvent;

/**
 * 遥控器控制器类
 * 处理TV遥控器、键盘、手柄的按键映射
 * 
 * 支持6个按键:
 * - 上键: 菜单向上/游戏中跳跃
 * - 下键: 菜单向下/游戏中向下(钻管道)/发射子弹
 * - 左键: 左行走
 * - 右键: 右行走
 * - OK键: 菜单确认/游戏中跳跃
 * - 返回键: ESC
 * 
 * 特性:
 * - 自动加速模式: 检测到TV遥控器时自动开启
 * - 延迟释放: 左右行走按键松开后添加延迟，支持边走边跳
 */
public class RemoteController {
    private static final String TAG = "RemoteController";
    
    // 按钮常量 (与 Rust 代码保持一致)
    public static final int BTN_DPAD_LEFT = 1;
    public static final int BTN_DPAD_RIGHT = 2;
    public static final int BTN_DPAD_UP = 3;
    public static final int BTN_DPAD_DOWN = 4;
    public static final int BTN_A = 5;  // 跳跃
    public static final int BTN_B = 6;  // 加速
    public static final int BTN_X = 7;  // 发射
    public static final int BTN_Y = 8;
    
    // 左右方向键延迟释放时间 (毫秒)
    private static final long DIRECTION_RELEASE_DELAY_MS = 200;
    
    // 回调接口
    public interface InputCallback {
        // 检测到物理输入设备时回调 (用于隐藏虚拟按钮)
        void onPhysicalInputDetected();
    }
    
    // Handler用于延迟释放
    private Handler handler = new Handler(Looper.getMainLooper());
    
    // 当前按下的方向键状态
    private boolean leftPressed = false;
    private boolean rightPressed = false;
    private boolean upPressed = false;
    private boolean downPressed = false;
    
    // 延迟释放任务
    private Runnable leftReleaseRunnable;
    private Runnable rightReleaseRunnable;
    
    // 加速模式状态 (TV遥控器自动开启)
    private boolean accelerateMode = false;
    private boolean accelerateSent = false;  // 是否已发送加速按钮事件
    
    // 是否检测到TV遥控器输入
    private boolean tvRemoteDetected = false;
    
    // 输入回调
    private InputCallback inputCallback;
    
    /**
     * 设置输入回调
     */
    public void setInputCallback(InputCallback callback) {
        this.inputCallback = callback;
    }
    
    /**
     * 处理按键事件
     * 
     * @param event KeyEvent事件
     * @return true如果事件被处理, false交给系统处理
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
        
        // 音量键不拦截，交给系统处理
        if (keyCode == KeyEvent.KEYCODE_VOLUME_UP || keyCode == KeyEvent.KEYCODE_VOLUME_DOWN) {
            return false;
        }
        
        // 检测到物理输入，通知隐藏虚拟按钮
        notifyPhysicalInputDetected();
        
        // 检测是否为TV遥控器按键
        if (isTvRemoteKey(keyCode)) {
            handleTvRemoteDetected();
        }
        
        // 处理6个核心按键
        switch (keyCode) {
            // 上键: 菜单向上/游戏中跳跃
            case KeyEvent.KEYCODE_DPAD_UP:
                handleUpKey(pressed);
                return true;
            
            // 下键: 菜单向下/游戏中向下(钻管道)/发射子弹
            case KeyEvent.KEYCODE_DPAD_DOWN:
                handleDownKey(pressed);
                return true;
            
            // 左键: 左行走 (带延迟释放)
            case KeyEvent.KEYCODE_DPAD_LEFT:
                handleLeftKey(pressed);
                return true;
            
            // 右键: 右行走 (带延迟释放)
            case KeyEvent.KEYCODE_DPAD_RIGHT:
                handleRightKey(pressed);
                return true;
            
            // OK键: 菜单确认/游戏中跳跃
            case KeyEvent.KEYCODE_DPAD_CENTER:
            case KeyEvent.KEYCODE_ENTER:
                handleOkKey(pressed);
                return true;
            
            // 返回键: ESC
            case KeyEvent.KEYCODE_BACK:
                handleBackKey(pressed);
                return true;
            
            default:
                // 其他按键也标记为已处理，防止干扰
                Log.i(TAG, "[Remote] Ignored keyCode=" + keyCode);
                return true;
        }
    }
    
    /**
     * 判断是否为TV遥控器按键
     */
    private boolean isTvRemoteKey(int keyCode) {
        switch (keyCode) {
            case KeyEvent.KEYCODE_DPAD_UP:
            case KeyEvent.KEYCODE_DPAD_DOWN:
            case KeyEvent.KEYCODE_DPAD_LEFT:
            case KeyEvent.KEYCODE_DPAD_RIGHT:
            case KeyEvent.KEYCODE_DPAD_CENTER:
            case KeyEvent.KEYCODE_BACK:
                return true;
            default:
                return false;
        }
    }
    
    /**
     * 检测到TV遥控器，自动开启加速
     */
    private void handleTvRemoteDetected() {
        if (!tvRemoteDetected) {
            tvRemoteDetected = true;
            Log.i(TAG, "[Remote] TV remote detected, enabling auto-accelerate");
        }
        
        // 开启加速模式
        if (!accelerateMode) {
            accelerateMode = true;
            if (!accelerateSent) {
                accelerateSent = true;
                Log.i(TAG, "[Remote] Auto-accelerate ON");
                MainActivity.nativeOnButtonEvent(BTN_B, true);
            }
        }
    }
    
    /**
     * 通知检测到物理输入
     */
    private void notifyPhysicalInputDetected() {
        if (inputCallback != null) {
            inputCallback.onPhysicalInputDetected();
        }
    }
    
    // ========================================================================
    // 按键处理方法
    // 使用 nativeOnKeyEvent 发送原始 Android KeyCode 到 Rust 端
    // Rust 端的 joystick_android_tv.rs 模块会处理这些按键
    // ========================================================================
    
    /**
     * 处理上键
     * - 菜单: 向上移动
     * - 游戏: 跳跃 (Rust端 joystick_android_tv 模块处理)
     */
    private void handleUpKey(boolean pressed) {
        upPressed = pressed;
        Log.i(TAG, "[Remote] UP keyCode=19, pressed=" + pressed);
        
        // 发送原始 Android KeyCode 到 Rust
        // Rust 端会根据游戏状态决定是导航还是跳跃
        MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_DPAD_UP, pressed);
    }
    
    /**
     * 处理下键
     * - 菜单: 向下移动
     * - 游戏: 向下(钻管道) + 发射子弹 (Rust端处理)
     */
    private void handleDownKey(boolean pressed) {
        downPressed = pressed;
        Log.i(TAG, "[Remote] DOWN keyCode=20, pressed=" + pressed);
        
        // 发送原始 Android KeyCode 到 Rust
        MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_DPAD_DOWN, pressed);
    }
    
    /**
     * 处理左键 (带延迟释放)
     * 延迟释放是为了让遥控器能够边走边跳
     */
    private void handleLeftKey(boolean pressed) {
        // 取消之前的延迟释放任务
        if (leftReleaseRunnable != null) {
            handler.removeCallbacks(leftReleaseRunnable);
            leftReleaseRunnable = null;
        }
        
        if (pressed) {
            // 按下时立即生效
            if (!leftPressed) {
                leftPressed = true;
                Log.i(TAG, "[Remote] LEFT keyCode=21, pressed=true");
                MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_DPAD_LEFT, true);
            }
            
            // 如果按下左键，立即取消右键的延迟状态并释放
            if (rightPressed) {
                if (rightReleaseRunnable != null) {
                    handler.removeCallbacks(rightReleaseRunnable);
                    rightReleaseRunnable = null;
                }
                rightPressed = false;
                MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_DPAD_RIGHT, false);
                Log.i(TAG, "[Remote] RIGHT cancelled due to LEFT press");
            }
        } else {
            // 松开时延迟释放
            leftReleaseRunnable = () -> {
                if (leftPressed) {
                    leftPressed = false;
                    Log.i(TAG, "[Remote] LEFT released (delayed)");
                    MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_DPAD_LEFT, false);
                }
                leftReleaseRunnable = null;
            };
            handler.postDelayed(leftReleaseRunnable, DIRECTION_RELEASE_DELAY_MS);
            Log.i(TAG, "[Remote] LEFT release delayed " + DIRECTION_RELEASE_DELAY_MS + "ms");
        }
    }
    
    /**
     * 处理右键 (带延迟释放)
     * 延迟释放是为了让遥控器能够边走边跳
     */
    private void handleRightKey(boolean pressed) {
        // 取消之前的延迟释放任务
        if (rightReleaseRunnable != null) {
            handler.removeCallbacks(rightReleaseRunnable);
            rightReleaseRunnable = null;
        }
        
        if (pressed) {
            // 按下时立即生效
            if (!rightPressed) {
                rightPressed = true;
                Log.i(TAG, "[Remote] RIGHT keyCode=22, pressed=true");
                MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_DPAD_RIGHT, true);
            }
            
            // 如果按下右键，立即取消左键的延迟状态并释放
            if (leftPressed) {
                if (leftReleaseRunnable != null) {
                    handler.removeCallbacks(leftReleaseRunnable);
                    leftReleaseRunnable = null;
                }
                leftPressed = false;
                MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_DPAD_LEFT, false);
                Log.i(TAG, "[Remote] LEFT cancelled due to RIGHT press");
            }
        } else {
            // 松开时延迟释放
            rightReleaseRunnable = () -> {
                if (rightPressed) {
                    rightPressed = false;
                    Log.i(TAG, "[Remote] RIGHT released (delayed)");
                    MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_DPAD_RIGHT, false);
                }
                rightReleaseRunnable = null;
            };
            handler.postDelayed(rightReleaseRunnable, DIRECTION_RELEASE_DELAY_MS);
            Log.i(TAG, "[Remote] RIGHT release delayed " + DIRECTION_RELEASE_DELAY_MS + "ms");
        }
    }
    
    /**
     * 处理OK键
     * - 菜单: 确认选择
     * - 游戏: 跳跃
     * 发送原始 KeyCode，Rust端会处理跳跃和确认逻辑
     */
    private void handleOkKey(boolean pressed) {
        Log.i(TAG, "[Remote] OK keyCode=23, pressed=" + pressed);
        
        // 发送原始 Android KeyCode 到 Rust
        MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_DPAD_CENTER, pressed);
    }
    
    /**
     * 处理返回键
     * - 作为ESC键使用
     */
    private void handleBackKey(boolean pressed) {
        Log.i(TAG, "[Remote] BACK keyCode=4, pressed=" + pressed);
        
        // 发送原始 Android KeyCode 到 Rust
        MainActivity.nativeOnKeyEvent(KeyEvent.KEYCODE_BACK, pressed);
    }
    
    // ========================================================================
    // 公共方法
    // ========================================================================
    
    /**
     * 检查是否已检测到TV遥控器
     */
    public boolean isTvRemoteDetected() {
        return tvRemoteDetected;
    }
    
    /**
     * 检查加速模式是否开启
     */
    public boolean isAccelerateModeOn() {
        return accelerateMode;
    }
    
    /**
     * 手动切换加速模式
     */
    public void toggleAccelerateMode() {
        accelerateMode = !accelerateMode;
        Log.i(TAG, "[Remote] Accelerate toggle: " + (accelerateMode ? "ON" : "OFF"));
        
        if (accelerateMode) {
            if (!accelerateSent) {
                accelerateSent = true;
                MainActivity.nativeOnButtonEvent(BTN_B, true);
            }
        } else {
            if (accelerateSent) {
                accelerateSent = false;
                MainActivity.nativeOnButtonEvent(BTN_B, false);
            }
        }
    }
    
    /**
     * 释放所有按键 (Activity暂停时调用)
     */
    public void releaseAllKeys() {
        // 取消所有延迟任务
        if (leftReleaseRunnable != null) {
            handler.removeCallbacks(leftReleaseRunnable);
            leftReleaseRunnable = null;
        }
        if (rightReleaseRunnable != null) {
            handler.removeCallbacks(rightReleaseRunnable);
            rightReleaseRunnable = null;
        }
        
        // 释放所有方向键
        if (leftPressed) {
            leftPressed = false;
            MainActivity.nativeOnButtonEvent(BTN_DPAD_LEFT, false);
        }
        if (rightPressed) {
            rightPressed = false;
            MainActivity.nativeOnButtonEvent(BTN_DPAD_RIGHT, false);
        }
        if (upPressed) {
            upPressed = false;
            MainActivity.nativeOnButtonEvent(BTN_DPAD_UP, false);
            MainActivity.nativeOnButtonEvent(BTN_A, false);
        }
        if (downPressed) {
            downPressed = false;
            MainActivity.nativeOnButtonEvent(BTN_DPAD_DOWN, false);
            MainActivity.nativeOnButtonEvent(BTN_X, false);
        }
        
        Log.i(TAG, "[Remote] All keys released");
    }
}
