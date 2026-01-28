package com.mariogame.mario;

import android.os.Bundle;
import android.util.DisplayMetrics;
import android.util.Log;
import android.view.KeyEvent;
import android.view.ViewGroup;
import android.widget.FrameLayout;

import com.google.androidgamesdk.GameActivity;

/**
 * 自定义 MainActivity
 * 继承 GameActivity, 使用模块化的控制器来处理输入
 * 
 * 控制器分离:
 * - VirtualController: 处理屏幕虚拟触摸按钮
 * - RemoteController: 处理TV遥控器/键盘/手柄按键映射
 * 
 * TV遥控器特性:
 * - 只使用6个按键: 上下左右OK返回
 * - 自动开启加速模式
 * - 左右行走延迟释放(支持边走边跳)
 * - 检测到物理输入时自动隐藏虚拟按钮
 */
public class MainActivity extends GameActivity {
    private static final String TAG = "MarioRS";
    
    // 屏幕尺寸
    private int screenWidth, screenHeight;
    
    // 控制器
    private VirtualController virtualController;
    private RemoteController remoteController;
    
    // Native 方法声明 - 由 Rust 实现
    public static native void nativeOnButtonEvent(int buttonId, boolean pressed);
    public static native void nativeOnKeyEvent(int keyCode, boolean pressed);
    
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        
        // 获取屏幕尺寸
        DisplayMetrics metrics = new DisplayMetrics();
        getWindowManager().getDefaultDisplay().getRealMetrics(metrics);
        screenWidth = metrics.widthPixels;
        screenHeight = metrics.heightPixels;
        
        Log.i(TAG, "Screen size: " + screenWidth + "x" + screenHeight);
        
        // 初始化控制器
        virtualController = new VirtualController(this, screenWidth, screenHeight);
        remoteController = new RemoteController();
        
        // 设置遥控器输入回调 (检测到物理输入时隐藏虚拟按钮)
        remoteController.setInputCallback(new RemoteController.InputCallback() {
            @Override
            public void onPhysicalInputDetected() {
                if (virtualController != null) {
                    virtualController.hideGameButtons();
                }
            }
        });
        
        // 延迟添加按钮层, 等待 GameActivity 的 SurfaceView 初始化完成
        getWindow().getDecorView().post(this::addButtonOverlay);
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
        
        // 使用VirtualController创建按钮覆盖层
        FrameLayout buttonLayer = virtualController.createButtonOverlay(contentView);
        contentView.addView(buttonLayer);
        
        Log.i(TAG, "Button overlay added");
    }
    
    /**
     * 拦截物理键盘事件并转发到遥控器控制器
     */
    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        // 使用RemoteController处理按键事件
        if (remoteController.handleKeyEvent(event)) {
            return true;
        }
        
        return super.dispatchKeyEvent(event);
    }
    
    @Override
    protected void onPause() {
        super.onPause();
        
        // 释放所有按键状态
        if (remoteController != null) {
            remoteController.releaseAllKeys();
        }
    }
    
    @Override
    protected void onDestroy() {
        super.onDestroy();
        
        // 清理资源
        if (remoteController != null) {
            remoteController.releaseAllKeys();
        }
    }
}
