package com.mariogame.mario;

import android.content.Context;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.util.DisplayMetrics;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.KeyEvent;
import android.widget.FrameLayout;
import android.widget.TextView;
import android.graphics.Color;
import android.view.Gravity;

import com.google.androidgamesdk.GameActivity;
import com.mariogame.R;

/**
 * 自定义 MainActivity
 * 继承 GameActivity, 添加原生按钮覆盖层解决多点触摸延迟问题
 * 自定义虚拟键盘面板用于输入作弊码
 * 支持 Android TV 遥控器控制游戏
 * 
 * TV遥控器横向握持模式：
 * - D-Pad方向已反转（上下->左右，左右->上下）
 * - 音量键保留给系统控制音量
 */
public class MainActivity extends GameActivity {
    private static final String TAG = "MarioRS";
    private static final String PREFS_NAME = "MarioButtonLayout";
    
    // 按钮常量 (与 Rust 代码保持一致)
    public static final int BTN_DPAD_LEFT = 1;
    public static final int BTN_DPAD_RIGHT = 2;
    public static final int BTN_DPAD_UP = 3;
    public static final int BTN_DPAD_DOWN = 4;
    public static final int BTN_A = 5;
    public static final int BTN_B = 6;
    public static final int BTN_X = 7;
    public static final int BTN_Y = 8;
    
    // ============================================================================
    // Android TV 遥控器按键码常量
    // ============================================================================
    
    // TV遥控器彩色按键
    private static final int KEYCODE_PROG_RED = 183;
    private static final int KEYCODE_PROG_GREEN = 184;
    private static final int KEYCODE_PROG_YELLOW = 185;
    private static final int KEYCODE_PROG_BLUE = 186;
    
    // TV遥控器媒体控制键
    private static final int KEYCODE_MEDIA_PLAY_PAUSE = 85;
    private static final int KEYCODE_MEDIA_PLAY = 126;
    private static final int KEYCODE_MEDIA_FAST_FORWARD = 90;
    private static final int KEYCODE_MEDIA_REWIND = 89;
    
    // TV遥控器频道键
    private static final int KEYCODE_CHANNEL_UP = 166;
    private static final int KEYCODE_CHANNEL_DOWN = 167;
    
    // 游戏手柄按钮 (Fire TV等)
    private static final int KEYCODE_BUTTON_A = 96;
    private static final int KEYCODE_BUTTON_B = 97;
    private static final int KEYCODE_BUTTON_X = 99;
    private static final int KEYCODE_BUTTON_Y = 100;
    
    // ============================================================================
    // 遥控器方向模式配置
    // ============================================================================
    
    // 是否启用横向遥控器模式 (D-pad方向反转: 上下<->左右)
    // true: 横向握持遥控器，上=左，下=右，左=下，右=上
    // false: 正常模式
    private static final boolean HORIZONTAL_REMOTE_MODE = true;
    
    // 通用遥控器功能键
    private static final int KEYCODE_PAGE_UP = 92;    // 小箭头上键
    private static final int KEYCODE_PAGE_DOWN = 93;  // 小箭头下键
    private static final int KEYCODE_MENU = 82;       // Menu键
    
    // 虚拟键盘按键映射 (View ID -> KeyCode)
    private static final int[][] VKBD_KEY_MAP = {
        {R.id.vkbd_key_p, KeyEvent.KEYCODE_P},
        {R.id.vkbd_key_tab, KeyEvent.KEYCODE_TAB},
        {R.id.vkbd_key_0, KeyEvent.KEYCODE_0},
        {R.id.vkbd_key_1, KeyEvent.KEYCODE_1},
        {R.id.vkbd_key_2, KeyEvent.KEYCODE_2},
        {R.id.vkbd_key_3, KeyEvent.KEYCODE_3},
        {R.id.vkbd_key_4, KeyEvent.KEYCODE_4},
        {R.id.vkbd_key_5, KeyEvent.KEYCODE_5},
        {R.id.vkbd_key_6, KeyEvent.KEYCODE_6},
        {R.id.vkbd_key_7, KeyEvent.KEYCODE_7},
        {R.id.vkbd_key_8, KeyEvent.KEYCODE_8},
        {R.id.vkbd_key_9, KeyEvent.KEYCODE_9},
        {R.id.vkbd_key_a, KeyEvent.KEYCODE_A},
        {R.id.vkbd_key_b, KeyEvent.KEYCODE_B},
        {R.id.vkbd_key_c, KeyEvent.KEYCODE_C},
        {R.id.vkbd_key_d, KeyEvent.KEYCODE_D},
        {R.id.vkbd_key_e, KeyEvent.KEYCODE_E},
        {R.id.vkbd_key_f, KeyEvent.KEYCODE_F},
        {R.id.vkbd_key_ent, KeyEvent.KEYCODE_ENTER},
    };
    
    // 按钮信息结构
    private static class ButtonInfo {
        View view;
        int id;
        int viewId;
        int defaultX, defaultY;
        int width, height;
        int backgroundResId;
        
        ButtonInfo(int id, int viewId, int backgroundResId) {
            this.id = id;
            this.viewId = viewId;
            this.backgroundResId = backgroundResId;
        }
    }
    
    // 所有游戏按钮
    private ButtonInfo[] gameButtons;
    // 功能按钮
    private View btnEdit;
    private TextView btnKeyboard;
    private TextView btnHide;  // 隐藏/显示游戏按钮
    private boolean gameButtonsVisible = true;  // 游戏按钮是否可见
    // 虚拟键盘面板
    private FrameLayout vkbdPanel;
    private boolean vkbdVisible = false;
    
    // 屏幕尺寸
    private int screenWidth, screenHeight;
    
    // 编辑模式
    private boolean editMode = false;
    
    // ============================================================================
    // 遥控器加速模式切换 (Toggle模式)
    // ============================================================================
    
    // 加速模式状态 (true=加速中, false=正常速度)
    // 通过 MENU 键或其他加速按钮切换
    private boolean accelerateMode = false;
    
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
        
        // 从 XML 加载按钮布局
        LayoutInflater inflater = LayoutInflater.from(this);
        FrameLayout buttonLayer = (FrameLayout) inflater.inflate(R.layout.button_overlay, contentView, false);
        
        // 计算按钮尺寸 (基于屏幕高度)
        int btnSize = screenHeight / 5;
        int dpadSize = btnSize;
        int margin = screenHeight / 20;
        int spacing = btnSize / 8;
        int smallBtnSize = btnSize / 2;
        // 右侧额外边距 (避免被屏幕边缘遮挡)
        int rightExtraMargin = screenWidth / 20;
        
        // D-Pad 中心位置
        int dpadCenterX = margin + dpadSize;
        int dpadCenterY = screenHeight - margin - dpadSize;
        
        // 右侧按钮中心位置 (增加额外边距)
        int rightCenterX = screenWidth - margin - btnSize - rightExtraMargin;
        int rightCenterY = screenHeight - margin - btnSize;
        
        // 初始化按钮信息 (只有 D-Pad 和 A/B/X/Y)
        gameButtons = new ButtonInfo[] {
            new ButtonInfo(BTN_DPAD_LEFT, R.id.btn_dpad_left, R.drawable.button_dpad),
            new ButtonInfo(BTN_DPAD_RIGHT, R.id.btn_dpad_right, R.drawable.button_dpad),
            new ButtonInfo(BTN_DPAD_UP, R.id.btn_dpad_up, R.drawable.button_dpad),
            new ButtonInfo(BTN_DPAD_DOWN, R.id.btn_dpad_down, R.drawable.button_dpad),
            new ButtonInfo(BTN_A, R.id.btn_a, R.drawable.button_a),
            new ButtonInfo(BTN_B, R.id.btn_b, R.drawable.button_b),
            new ButtonInfo(BTN_X, R.id.btn_x, R.drawable.button_x),
            new ButtonInfo(BTN_Y, R.id.btn_y, R.drawable.button_y),
        };
        
        // 计算默认位置
        int[][] defaultPositions = {
            {dpadCenterX - dpadSize, dpadCenterY - dpadSize/2, dpadSize, dpadSize},      // LEFT
            {dpadCenterX, dpadCenterY - dpadSize/2, dpadSize, dpadSize},                  // RIGHT
            {dpadCenterX - dpadSize/2, dpadCenterY - dpadSize, dpadSize, dpadSize},       // UP
            {dpadCenterX - dpadSize/2, dpadCenterY, dpadSize, dpadSize},                  // DOWN
            {rightCenterX, rightCenterY - btnSize/2, btnSize, btnSize},                   // A
            {rightCenterX - btnSize/2 - spacing/2, rightCenterY + spacing, btnSize, btnSize}, // B
            {rightCenterX - btnSize - spacing, rightCenterY - btnSize/2, btnSize, btnSize},   // X
            {rightCenterX - btnSize/2 - spacing/2, rightCenterY - btnSize - spacing, btnSize, btnSize}, // Y
        };
        
        // 加载保存的位置
        SharedPreferences prefs = getSharedPreferences(PREFS_NAME, MODE_PRIVATE);
        
        // 设置游戏按钮
        for (int i = 0; i < gameButtons.length; i++) {
            ButtonInfo info = gameButtons[i];
            info.view = buttonLayer.findViewById(info.viewId);
            
            // 设置默认尺寸和位置
            info.width = defaultPositions[i][2];
            info.height = defaultPositions[i][3];
            info.defaultX = prefs.getInt("btn_" + info.id + "_x", defaultPositions[i][0]);
            info.defaultY = prefs.getInt("btn_" + info.id + "_y", defaultPositions[i][1]);
            
            // 应用布局参数
            FrameLayout.LayoutParams params = new FrameLayout.LayoutParams(info.width, info.height);
            params.leftMargin = info.defaultX;
            params.topMargin = info.defaultY;
            info.view.setLayoutParams(params);
            
            // 设置触摸事件
            final ButtonInfo finalInfo = info;
            info.view.setOnTouchListener((v, event) -> {
                if (editMode) {
                    return handleEditTouch(finalInfo, v, event);
                } else {
                    return handleGameTouch(finalInfo.id, v, event);
                }
            });
        }
        
        // 隐藏暂停按钮 (不再需要)
        View btnPause = buttonLayer.findViewById(R.id.btn_pause);
        if (btnPause != null) {
            btnPause.setVisibility(View.GONE);
        }
        
        // 隐藏作弊码按钮 (不再需要)
        View btnCheat = buttonLayer.findViewById(R.id.btn_cheat);
        if (btnCheat != null) {
            btnCheat.setVisibility(View.GONE);
        }
        
        // 设置编辑按钮 (增加右侧边距)
        btnEdit = buttonLayer.findViewById(R.id.btn_edit);
        FrameLayout.LayoutParams editParams = new FrameLayout.LayoutParams(smallBtnSize, smallBtnSize);
        editParams.leftMargin = screenWidth - margin - smallBtnSize * 2 - spacing - rightExtraMargin;
        editParams.topMargin = margin;
        btnEdit.setLayoutParams(editParams);
        btnEdit.setOnClickListener(v -> toggleEditMode());
        
        // 创建虚拟键盘按钮 (增加右侧边距, 使用 TextView 显示文字)
        btnKeyboard = new TextView(this);
        btnKeyboard.setBackgroundResource(R.drawable.button_keyboard);
        btnKeyboard.setText("KB");
        btnKeyboard.setTextColor(Color.WHITE);
        btnKeyboard.setTextSize(12);
        btnKeyboard.setGravity(Gravity.CENTER);
        FrameLayout.LayoutParams kbParams = new FrameLayout.LayoutParams(smallBtnSize, smallBtnSize);
        kbParams.leftMargin = screenWidth - margin - smallBtnSize - rightExtraMargin;
        kbParams.topMargin = margin;
        btnKeyboard.setLayoutParams(kbParams);
        btnKeyboard.setOnClickListener(v -> toggleVirtualKeyboard());
        buttonLayer.addView(btnKeyboard);
        
        // 创建隐藏/显示游戏按钮的按钮 (放在 E 按钮左边)
        btnHide = new TextView(this);
        btnHide.setBackgroundResource(R.drawable.button_edit);
        btnHide.setText("H");
        btnHide.setTextColor(Color.WHITE);
        btnHide.setTextSize(12);
        btnHide.setGravity(Gravity.CENTER);
        FrameLayout.LayoutParams hideParams = new FrameLayout.LayoutParams(smallBtnSize, smallBtnSize);
        hideParams.leftMargin = screenWidth - margin - smallBtnSize * 3 - spacing * 2 - rightExtraMargin;
        hideParams.topMargin = margin;
        btnHide.setLayoutParams(hideParams);
        btnHide.setOnClickListener(v -> toggleGameButtons());
        buttonLayer.addView(btnHide);
        
        // 加载游戏按钮可见状态
        SharedPreferences prefs2 = getSharedPreferences(PREFS_NAME, MODE_PRIVATE);
        gameButtonsVisible = prefs2.getBoolean("game_buttons_visible", true);
        updateGameButtonsVisibility();
        
        // 创建虚拟键盘面板 (必须使用 WRAP_CONTENT 避免填充整个屏幕)
        vkbdPanel = createVirtualKeyboardPanel();
        vkbdPanel.setVisibility(View.GONE);
        FrameLayout.LayoutParams vkbdParams = new FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.WRAP_CONTENT,
            FrameLayout.LayoutParams.WRAP_CONTENT);
        buttonLayer.addView(vkbdPanel, vkbdParams);
        
        contentView.addView(buttonLayer);
        Log.i(TAG, "Button overlay added");
    }
    
    /**
     * 创建虚拟键盘面板 (从 XML 加载)
     */
    private FrameLayout createVirtualKeyboardPanel() {
        // 从 XML 加载面板布局 (使用临时 FrameLayout 作为 parent 以保留 wrap_content)
        LayoutInflater inflater = LayoutInflater.from(this);
        FrameLayout tempParent = new FrameLayout(this);
        FrameLayout panel = (FrameLayout) inflater.inflate(R.layout.vkbd_panel, tempParent, false);
        
        // 为每个按键设置触摸事件
        for (int[] mapping : VKBD_KEY_MAP) {
            int viewId = mapping[0];
            int keyCode = mapping[1];
            
            View keyView = panel.findViewById(viewId);
            if (keyView != null) {
                setupKeyTouchListener(keyView, keyCode);
            }
        }
        
        // 为面板背景设置拖动触摸事件 (按住空白区域可拖动)
        panel.setOnTouchListener((v, event) -> {
            int action = event.getActionMasked();
            FrameLayout.LayoutParams params = (FrameLayout.LayoutParams) v.getLayoutParams();
            
            switch (action) {
                case MotionEvent.ACTION_DOWN:
                    vkbdDragStartX = event.getRawX();
                    vkbdDragStartY = event.getRawY();
                    vkbdDragStartMarginX = params.leftMargin;
                    vkbdDragStartMarginY = params.topMargin;
                    return true;
                    
                case MotionEvent.ACTION_MOVE:
                    float dx = event.getRawX() - vkbdDragStartX;
                    float dy = event.getRawY() - vkbdDragStartY;
                    
                    int newX = (int)(vkbdDragStartMarginX + dx);
                    int newY = (int)(vkbdDragStartMarginY + dy);
                    
                    // 限制在屏幕范围内
                    int panelW = v.getWidth();
                    int panelH = v.getHeight();
                    newX = Math.max(0, Math.min(screenWidth - panelW, newX));
                    newY = Math.max(0, Math.min(screenHeight - panelH, newY));
                    
                    params.leftMargin = newX;
                    params.topMargin = newY;
                    v.setLayoutParams(params);
                    return true;
                    
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    // 保存虚拟键盘位置
                    saveVkbdPosition(params.leftMargin, params.topMargin);
                    return true;
            }
            return false;
        });
        
        return panel;
    }
    
    /**
     * 保存虚拟键盘面板位置
     */
    private void saveVkbdPosition(int x, int y) {
        SharedPreferences prefs = getSharedPreferences(PREFS_NAME, MODE_PRIVATE);
        SharedPreferences.Editor editor = prefs.edit();
        editor.putInt("vkbd_x", x);
        editor.putInt("vkbd_y", y);
        editor.apply();
    }
    
    /**
     * 更新虚拟键盘面板位置 (优先使用保存的位置, 否则居中显示)
     */
    private void updateVkbdPanelPosition() {
        if (vkbdPanel == null) return;
        
        vkbdPanel.post(() -> {
            int panelW = vkbdPanel.getWidth();
            int panelH = vkbdPanel.getHeight();
            
            // 如果尺寸还是 0，使用测量值
            if (panelW == 0 || panelH == 0) {
                vkbdPanel.measure(
                    View.MeasureSpec.makeMeasureSpec(screenWidth, View.MeasureSpec.AT_MOST),
                    View.MeasureSpec.makeMeasureSpec(screenHeight, View.MeasureSpec.AT_MOST));
                panelW = vkbdPanel.getMeasuredWidth();
                panelH = vkbdPanel.getMeasuredHeight();
            }
            
            // 从 SharedPreferences 加载保存的位置
            SharedPreferences prefs = getSharedPreferences(PREFS_NAME, MODE_PRIVATE);
            int defaultX = (screenWidth - panelW) / 2;
            int defaultY = (screenHeight - panelH) / 3;
            
            int panelX = prefs.getInt("vkbd_x", defaultX);
            int panelY = prefs.getInt("vkbd_y", defaultY);
            
            // 确保不会超出屏幕边界
            panelX = Math.max(0, Math.min(screenWidth - panelW, panelX));
            panelY = Math.max(0, Math.min(screenHeight - panelH, panelY));
            
            Log.i(TAG, "[VKBD] panel size=" + panelW + "x" + panelH + ", pos=(" + panelX + "," + panelY + ")");
            
            FrameLayout.LayoutParams params = (FrameLayout.LayoutParams) vkbdPanel.getLayoutParams();
            if (params == null) {
                params = new FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.WRAP_CONTENT,
                    FrameLayout.LayoutParams.WRAP_CONTENT);
            }
            params.leftMargin = panelX;
            params.topMargin = panelY;
            vkbdPanel.setLayoutParams(params);
        });
    }
    
    /**
     * 为虚拟键盘按键设置触摸监听器
     */
    private void setupKeyTouchListener(View keyView, int keyCode) {
        keyView.setOnTouchListener((v, event) -> {
            int action = event.getActionMasked();
            switch (action) {
                case MotionEvent.ACTION_DOWN:
                    Log.i(TAG, "[VKBD] keyCode=" + keyCode + ", pressed=true");
                    nativeOnKeyEvent(keyCode, true);
                    v.setAlpha(0.6f);
                    return true;
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    Log.i(TAG, "[VKBD] keyCode=" + keyCode + ", pressed=false");
                    nativeOnKeyEvent(keyCode, false);
                    v.setAlpha(1.0f);
                    return true;
            }
            return false;
        });
    }
    
    /**
     * 切换虚拟键盘面板显示/隐藏
     */
    private void toggleVirtualKeyboard() {
        vkbdVisible = !vkbdVisible;
        vkbdPanel.setVisibility(vkbdVisible ? View.VISIBLE : View.GONE);
        btnKeyboard.setAlpha(vkbdVisible ? 0.5f : 1.0f);
        Log.i(TAG, "[toggleVirtualKeyboard] visible=" + vkbdVisible);
        
        // 显示时更新位置
        if (vkbdVisible) {
            updateVkbdPanelPosition();
        }
    }
    
    /**
     * 切换游戏按钮显示/隐藏
     */
    private void toggleGameButtons() {
        gameButtonsVisible = !gameButtonsVisible;
        updateGameButtonsVisibility();
        
        // 保存状态
        SharedPreferences prefs = getSharedPreferences(PREFS_NAME, MODE_PRIVATE);
        prefs.edit().putBoolean("game_buttons_visible", gameButtonsVisible).apply();
        
        Log.i(TAG, "[toggleGameButtons] visible=" + gameButtonsVisible);
    }
    
    /**
     * 更新游戏按钮可见性
     */
    private void updateGameButtonsVisibility() {
        int visibility = gameButtonsVisible ? View.VISIBLE : View.GONE;
        for (ButtonInfo info : gameButtons) {
            if (info.view != null) {
                info.view.setVisibility(visibility);
            }
        }
        // 更新 H 按钮透明度表示当前状态
        btnHide.setAlpha(gameButtonsVisible ? 1.0f : 0.5f);
    }
    
    /**
     * 处理游戏触摸事件
     */
    private boolean handleGameTouch(int buttonId, View v, MotionEvent event) {
        int action = event.getActionMasked();
        switch (action) {
            case MotionEvent.ACTION_DOWN:
            case MotionEvent.ACTION_POINTER_DOWN:
                nativeOnButtonEvent(buttonId, true);
                v.setAlpha(0.6f);
                return true;
                
            case MotionEvent.ACTION_UP:
            case MotionEvent.ACTION_POINTER_UP:
            case MotionEvent.ACTION_CANCEL:
                nativeOnButtonEvent(buttonId, false);
                v.setAlpha(1.0f);
                return true;
        }
        return false;
    }
    
    // 编辑模式拖动相关
    private float dragStartX, dragStartY;
    private int dragStartMarginX, dragStartMarginY;
    
    // 虚拟键盘面板拖动相关
    private float vkbdDragStartX, vkbdDragStartY;
    private int vkbdDragStartMarginX, vkbdDragStartMarginY;
    
    /**
     * 处理编辑模式拖动
     */
    private boolean handleEditTouch(ButtonInfo info, View v, MotionEvent event) {
        int action = event.getActionMasked();
        FrameLayout.LayoutParams params = (FrameLayout.LayoutParams) v.getLayoutParams();
        
        switch (action) {
            case MotionEvent.ACTION_DOWN:
                dragStartX = event.getRawX();
                dragStartY = event.getRawY();
                dragStartMarginX = params.leftMargin;
                dragStartMarginY = params.topMargin;
                v.setAlpha(0.7f);
                return true;
                
            case MotionEvent.ACTION_MOVE:
                float dx = event.getRawX() - dragStartX;
                float dy = event.getRawY() - dragStartY;
                
                int newX = (int)(dragStartMarginX + dx);
                int newY = (int)(dragStartMarginY + dy);
                
                // 限制在屏幕范围内
                newX = Math.max(0, Math.min(screenWidth - info.width, newX));
                newY = Math.max(0, Math.min(screenHeight - info.height, newY));
                
                params.leftMargin = newX;
                params.topMargin = newY;
                v.setLayoutParams(params);
                return true;
                
            case MotionEvent.ACTION_UP:
            case MotionEvent.ACTION_CANCEL:
                // 保存新位置
                info.defaultX = params.leftMargin;
                info.defaultY = params.topMargin;
                v.setAlpha(1.0f);
                saveButtonPositions();
                return true;
        }
        return false;
    }
    
    /**
     * 切换编辑模式
     */
    private void toggleEditMode() {
        editMode = !editMode;
        
        if (editMode) {
            btnEdit.setAlpha(0.5f);
            for (ButtonInfo info : gameButtons) {
                info.view.setBackgroundResource(R.drawable.button_edit_highlight);
            }
        } else {
            btnEdit.setAlpha(1.0f);
            for (ButtonInfo info : gameButtons) {
                info.view.setBackgroundResource(info.backgroundResId);
            }
        }
    }
    
    /**
     * 保存按钮位置
     */
    private void saveButtonPositions() {
        SharedPreferences prefs = getSharedPreferences(PREFS_NAME, MODE_PRIVATE);
        SharedPreferences.Editor editor = prefs.edit();
        
        for (ButtonInfo info : gameButtons) {
            editor.putInt("btn_" + info.id + "_x", info.defaultX);
            editor.putInt("btn_" + info.id + "_y", info.defaultY);
        }
        
        editor.apply();
    }
    
    /**
     * 拦截物理键盘事件并转发到 Rust
     * 包含 Android TV 遥控器按键映射
     */
    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        int keyCode = event.getKeyCode();
        int action = event.getAction();
        
        // 只处理按下和抬起事件
        if (action == KeyEvent.ACTION_DOWN || action == KeyEvent.ACTION_UP) {
            boolean pressed = (action == KeyEvent.ACTION_DOWN);
            
            // 忽略重复按键事件 (长按时会产生)
            if (pressed && event.getRepeatCount() > 0) {
                return true;
            }
            
            // 音量键不拦截，交给系统处理
            if (keyCode == KeyEvent.KEYCODE_VOLUME_UP || keyCode == KeyEvent.KEYCODE_VOLUME_DOWN) {
                return super.dispatchKeyEvent(event);
            }
            
            // 返回键特殊处理: 传递给Rust作为Escape键 (用于Intro界面返回菜单)
            if (keyCode == KeyEvent.KEYCODE_BACK) {
                Log.i(TAG, "[TV Remote] BACK -> Escape, pressed=" + pressed);
                nativeOnKeyEvent(keyCode, pressed);
                return true;
            }
            
            // ================================================================
            // 加速切换键处理 (Toggle模式)
            // MENU键/数字1键等: 点击切换加速状态，而不是按住
            // ================================================================
            if (isAccelerateToggleKey(keyCode)) {
                // 只在按下时切换状态，忽略抬起事件
                if (pressed) {
                    accelerateMode = !accelerateMode;
                    Log.i(TAG, "[TV Remote] Accelerate toggle: " + (accelerateMode ? "ON" : "OFF"));
                    // 发送加速按钮状态
                    nativeOnButtonEvent(BTN_B, accelerateMode);
                }
                return true;
            }
            
            // 尝试将按键映射为游戏按钮 (TV遥控器/手柄支持)
            int buttonId = mapKeyToGameButton(keyCode);
            if (buttonId != 0) {
                Log.i(TAG, "[TV Remote] keyCode=" + keyCode + " -> button=" + buttonId + ", pressed=" + pressed);
                nativeOnButtonEvent(buttonId, pressed);
                return true;
            }
            
            // 调试日志
            Log.i(TAG, "[dispatchKeyEvent] keyCode=" + keyCode + ", pressed=" + pressed);
            
            // 转发到 Rust 作为键盘事件
            nativeOnKeyEvent(keyCode, pressed);
            
            // 返回 true 表示已处理, 防止事件继续传递
            return true;
        }
        
        return super.dispatchKeyEvent(event);
    }
    
    /**
     * 检查按键是否为加速切换键
     * 这些按键使用Toggle模式: 点击一次开启加速，再点击一次关闭
     * 
     * @param keyCode Android KeyEvent 按键码
     * @return true 如果是加速切换键
     */
    private boolean isAccelerateToggleKey(int keyCode) {
        switch (keyCode) {
            case KeyEvent.KEYCODE_MENU:     // Menu键 (keyCode=82) - 主加速切换键
            case KeyEvent.KEYCODE_1:        // 数字1键
            case KEYCODE_BUTTON_B:          // 游戏手柄B按钮
            case KEYCODE_BUTTON_Y:          // 游戏手柄Y按钮
                return true;
            default:
                return false;
        }
    }
    
    /**
     * 将按键码映射为游戏按钮ID
     * 
     * 映射方案 (针对用户遥控器优化):
     * 
     * 方向键 (支持横向模式):
     * - 正常模式: D-Pad直接映射
     * - 横向模式: 上=左, 下=右, 左=下, 右=上
     * 
     * 功能按键:
     * - 跳跃: OK键(主) / Page Down键(遥控器小箭头下) / 红色键 / 播放键 / A按钮
     * - 发射: Page Up键(遥控器小箭头上) / 绿色键 / 快退键 / X按钮
     * - 加速(Toggle): Menu键 / 数字1键 / B按钮 / Y按钮 (点击切换，见isAccelerateToggleKey)
     * 
     * 系统按键 (不拦截):
     * - 音量键: 交给系统处理
     * - 返回键: 在dispatchKeyEvent中单独处理为Escape
     * 
     * @param keyCode Android KeyEvent 按键码
     * @return 游戏按钮ID, 0表示非游戏按钮
     */
    private int mapKeyToGameButton(int keyCode) {
        switch (keyCode) {
            // ================================================================
            // 方向键 (D-Pad) - 支持横向遥控器模式
            // ================================================================
            case KeyEvent.KEYCODE_DPAD_UP:
                // 横向模式: 上 -> 左
                return HORIZONTAL_REMOTE_MODE ? BTN_DPAD_LEFT : BTN_DPAD_UP;
            case KeyEvent.KEYCODE_DPAD_DOWN:
                // 横向模式: 下 -> 右
                return HORIZONTAL_REMOTE_MODE ? BTN_DPAD_RIGHT : BTN_DPAD_DOWN;
            case KeyEvent.KEYCODE_DPAD_LEFT:
                // 横向模式: 左 -> 下
                return HORIZONTAL_REMOTE_MODE ? BTN_DPAD_DOWN : BTN_DPAD_LEFT;
            case KeyEvent.KEYCODE_DPAD_RIGHT:
                // 横向模式: 右 -> 上
                return HORIZONTAL_REMOTE_MODE ? BTN_DPAD_UP : BTN_DPAD_RIGHT;
            
            // ================================================================
            // 跳跃按键 -> A按钮
            // OK键(主) / 红色键 / 播放键 / A按钮
            // 注: Menu键已改为加速切换键 (见isAccelerateToggleKey)
            // ================================================================
            case KeyEvent.KEYCODE_DPAD_CENTER:  // OK键 - 主跳跃键
            case KeyEvent.KEYCODE_ENTER:        // Enter键
            case KEYCODE_PROG_RED:              // 红色键
            case KEYCODE_MEDIA_PLAY_PAUSE:      // 播放/暂停键
            case KEYCODE_MEDIA_PLAY:            // 播放键
            case KEYCODE_MEDIA_FAST_FORWARD:    // 快进键
            case KEYCODE_BUTTON_A:              // 游戏手柄A按钮
                return BTN_A;
            
            // ================================================================
            // 发射按键 -> X按钮
            // Page Up键(主) / 绿色键 / 快退键 / X按钮
            // ================================================================
            case KeyEvent.KEYCODE_PAGE_UP:      // 遥控器小箭头上键 (keyCode=92) - 发射子弹
            case KEYCODE_PROG_GREEN:            // 绿色键
            case KEYCODE_MEDIA_REWIND:          // 快退键
            case KEYCODE_BUTTON_X:              // 游戏手柄X按钮
                return BTN_X;
            
            // ================================================================
            // 跳跃备选按键 -> A按钮
            // Page Down键 用于遥控器跳跃
            // ================================================================
            case KeyEvent.KEYCODE_PAGE_DOWN:    // 遥控器小箭头下键 (keyCode=93) - 跳跃
                return BTN_A;
            
            // ================================================================
            // 加速按键 -> 已移至Toggle模式处理
            // Menu键/数字1键/B按钮/Y按钮 现在通过 isAccelerateToggleKey() 处理
            // 点击一次开启加速，再点击一次关闭加速
            // ================================================================
            
            // ================================================================
            // 其他按键
            // ================================================================
            case KEYCODE_PROG_YELLOW:           // 黄色键 -> Y按钮 (可自定义)
                return BTN_Y;
            
            // 蓝色键 -> 不映射 (保留)
            case KEYCODE_PROG_BLUE:
                return 0;
            
            // 频道键可作为备选
            case KEYCODE_CHANNEL_UP:            // 频道上 -> 跳跃
                return BTN_A;
            case KEYCODE_CHANNEL_DOWN:          // 频道下 -> 发射
                return BTN_X;
            
            default:
                return 0;
        }
    }
}
