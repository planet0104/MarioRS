package com.mariogame.mario;

import android.content.Context;
import android.content.SharedPreferences;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.MotionEvent;
import android.view.View;
import android.view.ViewGroup;
import android.app.Activity;
import android.widget.FrameLayout;
import android.widget.TextView;
import android.graphics.Color;
import android.view.Gravity;

import com.mariogame.R;

/**
 * 虚拟控制器类
 * 处理屏幕上的虚拟触摸按钮
 * 支持按钮拖拽编辑和位置保存
 */
public class VirtualController {
    private static final String TAG = "VirtualController";
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
    
    // 虚拟键盘按键映射 (View ID -> KeyCode)
    private static final int[][] VKBD_KEY_MAP = {
        {R.id.vkbd_key_p, android.view.KeyEvent.KEYCODE_P},
        {R.id.vkbd_key_tab, android.view.KeyEvent.KEYCODE_TAB},
        {R.id.vkbd_key_0, android.view.KeyEvent.KEYCODE_0},
        {R.id.vkbd_key_1, android.view.KeyEvent.KEYCODE_1},
        {R.id.vkbd_key_2, android.view.KeyEvent.KEYCODE_2},
        {R.id.vkbd_key_3, android.view.KeyEvent.KEYCODE_3},
        {R.id.vkbd_key_4, android.view.KeyEvent.KEYCODE_4},
        {R.id.vkbd_key_5, android.view.KeyEvent.KEYCODE_5},
        {R.id.vkbd_key_6, android.view.KeyEvent.KEYCODE_6},
        {R.id.vkbd_key_7, android.view.KeyEvent.KEYCODE_7},
        {R.id.vkbd_key_8, android.view.KeyEvent.KEYCODE_8},
        {R.id.vkbd_key_9, android.view.KeyEvent.KEYCODE_9},
        {R.id.vkbd_key_a, android.view.KeyEvent.KEYCODE_A},
        {R.id.vkbd_key_b, android.view.KeyEvent.KEYCODE_B},
        {R.id.vkbd_key_c, android.view.KeyEvent.KEYCODE_C},
        {R.id.vkbd_key_d, android.view.KeyEvent.KEYCODE_D},
        {R.id.vkbd_key_e, android.view.KeyEvent.KEYCODE_E},
        {R.id.vkbd_key_f, android.view.KeyEvent.KEYCODE_F},
        {R.id.vkbd_key_ent, android.view.KeyEvent.KEYCODE_ENTER},
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
    
    // Context
    private Context context;
    private int screenWidth, screenHeight;
    
    // 所有游戏按钮
    private ButtonInfo[] gameButtons;
    
    // 功能按钮
    private View btnEdit;
    private TextView btnKeyboard;
    private TextView btnHide;
    
    // 按钮层容器
    private FrameLayout buttonLayer;
    
    // 虚拟键盘面板
    private FrameLayout vkbdPanel;
    private boolean vkbdVisible = false;
    
    // 编辑模式
    private boolean editMode = false;
    
    // 游戏按钮是否可见 (D-pad和A/B/X/Y按钮)
    // 由用户手动控制，持久化到 SharedPreferences
    private boolean gameButtonsVisible = true;
    
    // 整个控制器是否可见 (包括右上角功能按钮)
    // false: 整个控制器隐藏 (检测到遥控器时使用)
    // true: 控制器可见 (可以单独隐藏游戏按钮，但右上角按钮可见)
    // 注意: 此状态不持久化，每次启动默认可见，检测到遥控器时隐藏
    private boolean controllerVisible = true;
    
    // 是否已经检测到遥控器输入 (用于在 createButtonOverlay 中判断是否需要隐藏)
    private boolean remoteDetected = false;
    
    // 编辑模式拖动相关
    private float dragStartX, dragStartY;
    private int dragStartMarginX, dragStartMarginY;
    
    // 虚拟键盘面板拖动相关
    private float vkbdDragStartX, vkbdDragStartY;
    private int vkbdDragStartMarginX, vkbdDragStartMarginY;
    
    /**
     * 构造函数
     * 
     * @param context Activity上下文
     * @param screenWidth 屏幕宽度
     * @param screenHeight 屏幕高度
     */
    public VirtualController(Context context, int screenWidth, int screenHeight) {
        this.context = context;
        this.screenWidth = screenWidth;
        this.screenHeight = screenHeight;
    }
    
    /**
     * 创建并返回按钮覆盖层
     * 
     * @param contentView 父容器
     * @return 按钮覆盖层FrameLayout
     */
    public FrameLayout createButtonOverlay(ViewGroup contentView) {
        // 从 XML 加载按钮布局
        LayoutInflater inflater = LayoutInflater.from(context);
        buttonLayer = (FrameLayout) inflater.inflate(R.layout.button_overlay, contentView, false);
        
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
        SharedPreferences prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
        
        // 设置游戏按钮
        for (int i = 0; i < gameButtons.length; i++) {
            ButtonInfo info = gameButtons[i];
            info.view = buttonLayer.findViewById(info.viewId);
            
            // 禁用焦点，防止键盘/遥控器Enter键触发按钮
            info.view.setFocusable(false);
            info.view.setFocusableInTouchMode(false);
            
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
        // 禁用焦点，防止键盘/遥控器Enter键触发按钮
        btnEdit.setFocusable(false);
        btnEdit.setFocusableInTouchMode(false);
        FrameLayout.LayoutParams editParams = new FrameLayout.LayoutParams(smallBtnSize, smallBtnSize);
        editParams.leftMargin = screenWidth - margin - smallBtnSize * 2 - spacing - rightExtraMargin;
        editParams.topMargin = margin;
        btnEdit.setLayoutParams(editParams);
        btnEdit.setOnClickListener(v -> toggleEditMode());
        
        // 创建虚拟键盘按钮 (增加右侧边距, 使用 TextView 显示文字)
        btnKeyboard = new TextView(context);
        // 设置 id 以便在遍历视图树时能找到这个程序创建的控件
        btnKeyboard.setId(R.id.btn_keyboard);
        // 禁用焦点，防止键盘/遥控器Enter键触发按钮
        btnKeyboard.setFocusable(false);
        btnKeyboard.setFocusableInTouchMode(false);
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
        btnHide = new TextView(context);
        // 设置 id 以便在遍历视图树时能找到这个程序创建的控件
        btnHide.setId(R.id.btn_hide);
        // 禁用焦点，防止键盘/遥控器Enter键触发按钮
        btnHide.setFocusable(false);
        btnHide.setFocusableInTouchMode(false);
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
        
        // 加载游戏按钮可见状态 (不加载controllerVisible，因为遥控器检测应该每次都重新判断)
        gameButtonsVisible = prefs.getBoolean("game_buttons_visible", true);
        
        // 如果之前已经检测到遥控器输入，保持隐藏状态；否则默认可见
        if (!remoteDetected) {
            controllerVisible = true;
        }
        // 注意: 如果 remoteDetected = true，controllerVisible 已经在 hideGameButtons() 中被设置为 false
        
        Log.i(TAG, "[createButtonOverlay] remoteDetected=" + remoteDetected + 
            ", controllerVisible=" + controllerVisible + ", gameButtonsVisible=" + gameButtonsVisible);
        
        updateGameButtonsVisibility();
        
        // 创建虚拟键盘面板 (必须使用 WRAP_CONTENT 避免填充整个屏幕)
        vkbdPanel = createVirtualKeyboardPanel();
        vkbdPanel.setVisibility(View.GONE);
        FrameLayout.LayoutParams vkbdParams = new FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.WRAP_CONTENT,
            FrameLayout.LayoutParams.WRAP_CONTENT);
        buttonLayer.addView(vkbdPanel, vkbdParams);
        
        Log.i(TAG, "Button overlay created");
        return buttonLayer;
    }
    
    /**
     * 创建虚拟键盘面板 (从 XML 加载)
     */
    private FrameLayout createVirtualKeyboardPanel() {
        LayoutInflater inflater = LayoutInflater.from(context);
        FrameLayout tempParent = new FrameLayout(context);
        FrameLayout panel = (FrameLayout) inflater.inflate(R.layout.vkbd_panel, tempParent, false);
        
        // 为每个按键设置触摸事件
        for (int[] mapping : VKBD_KEY_MAP) {
            int viewId = mapping[0];
            int keyCode = mapping[1];
            
            View keyView = panel.findViewById(viewId);
            if (keyView != null) {
                // 禁用焦点，防止键盘/遥控器Enter键触发按钮
                keyView.setFocusable(false);
                keyView.setFocusableInTouchMode(false);
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
        SharedPreferences prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
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
            SharedPreferences prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
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
                    MainActivity.nativeOnKeyEvent(keyCode, true);
                    v.setAlpha(0.6f);
                    return true;
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    Log.i(TAG, "[VKBD] keyCode=" + keyCode + ", pressed=false");
                    MainActivity.nativeOnKeyEvent(keyCode, false);
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
        // 如果控制器不可见，不允许显示虚拟键盘
        if (!controllerVisible) {
            Log.w(TAG, "[toggleVirtualKeyboard] Controller is hidden, cannot show keyboard");
            return;
        }
        
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
     * 切换游戏按钮显示/隐藏 (触摸板单独隐藏，右上角按钮保持可见)
     */
    public void toggleGameButtons() {
        // 用户手动操作，重置遥控器检测状态，确保控制器可见
        remoteDetected = false;
        controllerVisible = true;
        gameButtonsVisible = !gameButtonsVisible;
        updateGameButtonsVisibility();
        
        // 保存状态
        SharedPreferences prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
        prefs.edit().putBoolean("game_buttons_visible", gameButtonsVisible).apply();
        
        Log.i(TAG, "[toggleGameButtons] gameButtonsVisible=" + gameButtonsVisible + ", controllerVisible=" + controllerVisible);
    }
    
    /**
     * 隐藏整个虚拟控制器 (当检测到物理输入设备时调用)
     * 彻底隐藏整个VirtualController，包括右上角功能按钮
     */
    public void hideGameButtons() {
        // 标记已检测到遥控器，即使 buttonLayer 还没创建也要记住这个状态
        remoteDetected = true;
        controllerVisible = false;
        
        // 如果 buttonLayer 已创建，立即更新可见性
        if (buttonLayer != null) {
            updateGameButtonsVisibility();
            Log.i(TAG, "[hideGameButtons] Controller hidden due to physical input detected");
        } else {
            Log.i(TAG, "[hideGameButtons] Remote detected, will hide when buttonLayer is created");
        }
    }
    
    /**
     * 更新游戏按钮可见性
     * 根据controllerVisible和gameButtonsVisible两个状态更新所有按钮的可见性
     */
    private void updateGameButtonsVisibility() {
        Log.i(TAG, "[updateGameButtonsVisibility] controllerVisible=" + controllerVisible + 
            ", gameButtonsVisible=" + gameButtonsVisible + ", buttonLayer=" + (buttonLayer != null));
        
        // 如果整个控制器隐藏，隐藏整个buttonLayer
        if (buttonLayer != null) {
            int layerVisibility = controllerVisible ? View.VISIBLE : View.GONE;
            buttonLayer.setVisibility(layerVisibility);
            Log.i(TAG, "[updateGameButtonsVisibility] buttonLayer.setVisibility(" + 
                (layerVisibility == View.VISIBLE ? "VISIBLE" : "GONE") + ")");
        }
        
        // 如果控制器不可见，隐藏虚拟键盘面板并返回
        if (!controllerVisible) {
            if (vkbdPanel != null) {
                vkbdPanel.setVisibility(View.GONE);
            }
            vkbdVisible = false;
            Log.i(TAG, "[updateGameButtonsVisibility] Controller hidden, returning early");
            return;
        }
        
        // 控制器可见时，根据gameButtonsVisible更新游戏按钮
        int gameButtonsVisibility = gameButtonsVisible ? View.VISIBLE : View.GONE;
        for (ButtonInfo info : gameButtons) {
            if (info.view != null) {
                info.view.setVisibility(gameButtonsVisibility);
            }
        }
        
        // 右上角功能按钮始终可见（当控制器可见时）
        if (btnEdit != null) {
            btnEdit.setVisibility(View.VISIBLE);
            btnEdit.setAlpha(gameButtonsVisible ? 1.0f : 0.5f);
        }
        if (btnKeyboard != null) {
            btnKeyboard.setVisibility(View.VISIBLE);
            btnKeyboard.setAlpha(gameButtonsVisible ? 1.0f : 0.5f);
        }
        if (btnHide != null) {
            btnHide.setVisibility(View.VISIBLE);
            btnHide.setAlpha(gameButtonsVisible ? 1.0f : 0.5f);
        }
    }
    
    /**
     * 处理游戏触摸事件
     */
    private boolean handleGameTouch(int buttonId, View v, MotionEvent event) {
        int action = event.getActionMasked();
        switch (action) {
            case MotionEvent.ACTION_DOWN:
            case MotionEvent.ACTION_POINTER_DOWN:
                MainActivity.nativeOnButtonEvent(buttonId, true);
                v.setAlpha(0.6f);
                return true;
                
            case MotionEvent.ACTION_UP:
            case MotionEvent.ACTION_POINTER_UP:
            case MotionEvent.ACTION_CANCEL:
                MainActivity.nativeOnButtonEvent(buttonId, false);
                v.setAlpha(1.0f);
                return true;
        }
        return false;
    }
    
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
        // 如果控制器不可见，不允许进入编辑模式
        if (!controllerVisible) {
            Log.w(TAG, "[toggleEditMode] Controller is hidden, cannot enter edit mode");
            return;
        }
        
        editMode = !editMode;
        
        if (editMode) {
            btnEdit.setAlpha(0.5f);
            for (ButtonInfo info : gameButtons) {
                if (info.view != null) {
                    info.view.setBackgroundResource(R.drawable.button_edit_highlight);
                }
            }
        } else {
            btnEdit.setAlpha(1.0f);
            for (ButtonInfo info : gameButtons) {
                if (info.view != null) {
                    info.view.setBackgroundResource(info.backgroundResId);
                }
            }
        }
    }
    
    /**
     * 保存按钮位置
     */
    private void saveButtonPositions() {
        SharedPreferences prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
        SharedPreferences.Editor editor = prefs.edit();
        
        for (ButtonInfo info : gameButtons) {
            editor.putInt("btn_" + info.id + "_x", info.defaultX);
            editor.putInt("btn_" + info.id + "_y", info.defaultY);
        }
        
        editor.apply();
    }
    
    /**
     * 检查游戏按钮是否可见
     */
    public boolean isGameButtonsVisible() {
        return gameButtonsVisible && controllerVisible;
    }
    
    /**
     * 检查整个控制器是否可见
     */
    public boolean isControllerVisible() {
        return controllerVisible;
    }
}
