// ============================================================================
// Mario RS - WeChat Mini Game Entry (CPU Rendering Version)
// ============================================================================

console.log('[game.js] Mario RS - WeChat Mini Game Starting (CPU Version)');

// ============================================================================
// Virtual Controller - 参考 Android VirtualController.java 实现
// 使用 Canvas 2D 叠加层显示虚拟按钮
// 
// 优化点:
// - 按钮状态缓存，减少遍历
// - 脏标记机制，避免无效重绘
// - 预计算按钮位置，减少运行时计算
// - 合并绘制调用
// ============================================================================
var VirtualController = {
    // 按钮常量 (与 Rust 代码保持一致)
    BTN_DPAD_LEFT: 1,
    BTN_DPAD_RIGHT: 2,
    BTN_DPAD_UP: 3,
    BTN_DPAD_DOWN: 4,
    BTN_A: 5,
    BTN_B: 6,
    BTN_X: 7,
    BTN_Y: 8,
    
    // 状态
    offscreenCanvas: null,
    ctx: null,
    mainCanvas: null,
    mainCtx: null,
    buttons: {},
    activeButtons: {},
    buttonStateCache: {},  // 优化：状态缓存
    screenWidth: 0,
    screenHeight: 0,
    visible: true,
    needsRedraw: true,     // 优化：脏标记
    audioResumed: false,
    
    // 性能优化：预计算的绘制参数
    buttonDrawParams: {},
    
    // 编辑模式
    editMode: false,
    editButton: null,
    dragInfo: null,
    
    // X按钮锁定状态（切换模式）
    xButtonLocked: false,
    
    // 虚拟键盘
    vkbdVisible: false,
    vkbdButton: null,
    vkbdPanel: null,
    vkbdKeys: [
        // Row 1: P, TAB, 0, 1, 2, 3, 4, 5, 6
        { label: 'P', code: 'KeyP', color: 'rgba(50, 100, 200, 0.45)' },
        { label: 'TAB', code: 'Tab', color: 'rgba(200, 50, 50, 0.45)' },
        { label: '0', code: 'Digit0', color: 'rgba(100, 100, 100, 0.45)' },
        { label: '1', code: 'Digit1', color: 'rgba(100, 100, 100, 0.45)' },
        { label: '2', code: 'Digit2', color: 'rgba(100, 100, 100, 0.45)' },
        { label: '3', code: 'Digit3', color: 'rgba(100, 100, 100, 0.45)' },
        { label: '4', code: 'Digit4', color: 'rgba(100, 100, 100, 0.45)' },
        { label: '5', code: 'Digit5', color: 'rgba(100, 100, 100, 0.45)' },
        { label: '6', code: 'Digit6', color: 'rgba(100, 100, 100, 0.45)' },
        // Row 2: 7, 8, 9, A, B, C, D, E, F, ENT
        { label: '7', code: 'Digit7', color: 'rgba(100, 100, 100, 0.45)' },
        { label: '8', code: 'Digit8', color: 'rgba(100, 100, 100, 0.45)' },
        { label: '9', code: 'Digit9', color: 'rgba(100, 100, 100, 0.45)' },
        { label: 'A', code: 'KeyA', color: 'rgba(100, 100, 100, 0.45)' },
        { label: 'B', code: 'KeyB', color: 'rgba(100, 100, 100, 0.45)' },
        { label: 'C', code: 'KeyC', color: 'rgba(100, 100, 100, 0.45)' },
        { label: 'D', code: 'KeyD', color: 'rgba(100, 100, 100, 0.45)' },
        { label: 'E', code: 'KeyE', color: 'rgba(100, 100, 100, 0.45)' },
        { label: 'F', code: 'KeyF', color: 'rgba(100, 100, 100, 0.45)' },
        { label: 'ENT', code: 'Enter', color: 'rgba(50, 200, 50, 0.45)' }
    ],
    vkbdPressedKey: null,
    editButtonClickCount: 0,  // 编辑按钮点击计数器（用于显示KB按钮）
    
    // 初始化
    init: function() {
        console.log('[VirtualController] Initializing (Optimized)...');
        
        var self = this;
        var info = wx.getSystemInfoSync();
        
        // 优化：适配设备像素比
        var dpr = info.pixelRatio || 1;
        this.screenWidth = info.screenWidth;
        this.screenHeight = info.screenHeight;
        
        console.log('[VirtualController] Screen size:', this.screenWidth, 'x', this.screenHeight, 'DPR:', dpr);
        
        // 创建离屏 2D Canvas 用于绘制虚拟按钮
        this.offscreenCanvas = wx.createCanvas();
        this.offscreenCanvas.width = this.screenWidth * dpr;
        this.offscreenCanvas.height = this.screenHeight * dpr;
        this.ctx = this.offscreenCanvas.getContext('2d');
        
        // 缩放上下文以适配DPR
        this.ctx.scale(dpr, dpr);
        
        // 计算按钮布局
        this.calculateButtonLayout();
        
        // 预计算绘制参数（优化）
        this.precalculateDrawParams();
        
        // 从本地存储加载按钮位置
        this.loadButtonPositions();
        
        // 创建编辑按钮（右上角）
        this.createEditButton();
        
        // 创建虚拟键盘按钮（右上角）
        this.createVkbdButton();
        
        // 创建虚拟键盘面板
        this.createVkbdPanel();
        
        // 注册触摸事件
        wx.onTouchStart(function(e) { self.onTouchStart(e); });
        wx.onTouchMove(function(e) { self.onTouchMove(e); });
        wx.onTouchEnd(function(e) { self.onTouchEnd(e); });
        wx.onTouchCancel(function(e) { self.onTouchEnd(e); });
        
        // 导出渲染函数供 Rust 调用
        GameGlobal.renderVirtualController = function() {
            self.render();
            self.renderOverlay();
        };
        
        // 导出 VirtualController 到 GameGlobal 供 Rust 调用 lateInit
        GameGlobal.VirtualController = this;
        
        console.log('[VirtualController] Initialized with', Object.keys(this.buttons).length, 'buttons');
    },
    
    // 计算按钮布局
    calculateButtonLayout: function() {
        var btnSize = Math.floor(this.screenHeight / 7);  // 缩小按钮尺寸以留出空间
        var dpadSize = btnSize;
        var margin = Math.floor(this.screenHeight / 20);
        var dpadSpacing = Math.floor(btnSize / 4);  // D-Pad专用间距
        var spacing = dpadSpacing;  // 右侧按钮使用与D-Pad相同的间距
        
        // D-Pad 中心位置 (左下角，向左偏移)
        var dpadCenterX = margin + dpadSize*1.5;  // 减小X坐标，往左移动
        var dpadCenterY = this.screenHeight - margin - dpadSize;
        var dpadSpacing = Math.floor(btnSize / 4);  // D-Pad专用间距
        
        // 右侧按钮中心位置 (右下角，向右移动)
        var rightCenterX = this.screenWidth - margin - btnSize;  // 减小margin，向右移动
        var rightCenterY = this.screenHeight - margin - btnSize;

        // 定义按钮
        this.buttons = {
            left: { 
                id: this.BTN_DPAD_LEFT, 
                x: dpadCenterX - dpadSize - dpadSpacing/2, 
                y: dpadCenterY - dpadSize/2, 
                w: dpadSize, 
                h: dpadSize,
                label: '<',
                color: 'rgba(100, 100, 100, 0.35)'
            },
            right: { 
                id: this.BTN_DPAD_RIGHT, 
                x: dpadCenterX + dpadSpacing/2, 
                y: dpadCenterY - dpadSize/2, 
                w: dpadSize, 
                h: dpadSize,
                label: '>',
                color: 'rgba(100, 100, 100, 0.35)'
            },
            up: { 
                id: this.BTN_DPAD_UP, 
                x: dpadCenterX - dpadSize/2, 
                y: dpadCenterY - dpadSize - dpadSpacing/2, 
                w: dpadSize, 
                h: dpadSize,
                label: '^',
                color: 'rgba(100, 100, 100, 0.35)'
            },
            down: { 
                id: this.BTN_DPAD_DOWN, 
                x: dpadCenterX - dpadSize/2, 
                y: dpadCenterY + dpadSpacing/2, 
                w: dpadSize, 
                h: dpadSize,
                label: 'v',
                color: 'rgba(100, 100, 100, 0.35)'
            },
            a: { 
                id: this.BTN_A, 
                x: rightCenterX, 
                y: rightCenterY - btnSize/2, 
                w: btnSize, 
                h: btnSize,
                label: 'A',
                color: 'rgba(0, 150, 0, 0.35)'
            },
            b: { 
                id: this.BTN_B, 
                x: rightCenterX - btnSize/2 - spacing/2, 
                y: rightCenterY + spacing, 
                w: btnSize, 
                h: btnSize,
                label: 'B',
                color: 'rgba(150, 0, 0, 0.35)'
            },
            x: { 
                id: this.BTN_X, 
                x: rightCenterX - btnSize - spacing, 
                y: rightCenterY - btnSize/2, 
                w: btnSize, 
                h: btnSize,
                label: 'X',
                color: 'rgba(0, 0, 150, 0.35)'
            },
            y: { 
                id: this.BTN_Y, 
                x: rightCenterX - btnSize/2 - spacing/2, 
                y: rightCenterY - btnSize - spacing, 
                w: btnSize, 
                h: btnSize,
                label: 'Y',
                color: 'rgba(150, 150, 0, 0.35)'
            }
        };
    },
    
    // 优化：预计算绘制参数
    precalculateDrawParams: function() {
        for (var name in this.buttons) {
            var btn = this.buttons[name];
            this.buttonDrawParams[name] = {
                centerX: btn.x + btn.w/2,
                centerY: btn.y + btn.h/2,
                radius: btn.w/2,
                fontSize: Math.floor(btn.w * 0.4)
            };
        }
    },
    
    // 从本地存储加载按钮位置
    loadButtonPositions: function() {
        try {
            var savedData = wx.getStorageSync('mario_button_layout');
            if (savedData) {
                for (var name in this.buttons) {
                    var btn = this.buttons[name];
                    var key = 'btn_' + btn.id;
                    if (savedData[key]) {
                        btn.x = savedData[key].x;
                        btn.y = savedData[key].y;
                    }
                }
                // 重新计算绘制参数
                this.precalculateDrawParams();
            }
        } catch (e) {
        }
    },
    
    // 保存按钮位置到本地存储
    saveButtonPositions: function() {
        var data = {};
        for (var name in this.buttons) {
            var btn = this.buttons[name];
            data['btn_' + btn.id] = { x: btn.x, y: btn.y };
        }
        try {
            wx.setStorageSync('mario_button_layout', data);
            
            // 显示保存成功提示
            wx.showToast({
                title: '布局已保存',
                icon: 'success',
                duration: 1500
            });
        } catch (e) {
        }
    },
    
    // 创建编辑按钮
    createEditButton: function() {
        var radius = 18;  // 圆形半径
        var margin = Math.floor(this.screenHeight / 20);
        var spacing = 8;
        
        this.editButton = {
            centerX: this.screenWidth - margin - radius,  // 靠右对齐
            centerY: margin + 40 + radius,  // 向下移动避免系统菜单遮挡（第一个按钮）
            radius: radius,
            label: '编辑',
            editLabel: '保存',
            color: 'rgba(100, 100, 200, 0.5)',
            activeColor: 'rgba(200, 100, 100, 0.5)'
        };
        
        console.log('[VirtualController] Edit button created at', this.editButton.centerX, this.editButton.centerY);
    },
    
    // 创建虚拟键盘按钮
    createVkbdButton: function() {
        var radius = 18;  // 圆形半径
        var margin = Math.floor(this.screenHeight / 20);
        var spacing = 8;
        
        this.vkbdButton = {
            centerX: this.screenWidth - margin - radius,  // 靠右对齐
            centerY: margin + 40 + radius * 3 + spacing,  // 排列在编辑按钮下方
            radius: radius,
            label: 'KB',
            color: 'rgba(100, 200, 100, 0.5)',
            activeColor: 'rgba(100, 200, 100, 0.7)',
            visible: false  // 默认隐藏，点击编辑按钮5次后显示
        };
        
        console.log('[VirtualController] Keyboard button created at', this.vkbdButton.centerX, this.vkbdButton.centerY);
    },
    
    // 创建虚拟键盘面板
    createVkbdPanel: function() {
        var keySize = 35;
        var keySpacing = 4;
        var cols = 10;  // 第一行10个按键（最多的一行）
        var rows = 2;   // 2行布局
        var padding = 8;
        var titleHeight = 20;
        
        var panelWidth = cols * (keySize + keySpacing) + padding * 2;
        var panelHeight = rows * (keySize + keySpacing) + padding * 2 + titleHeight;
        
        // 面板位置（屏幕中央靠下）
        var panelX = (this.screenWidth - panelWidth) / 2;
        var panelY = this.screenHeight * 0.65; // 屏幕下方约65%位置
        
        this.vkbdPanel = {
            x: panelX,
            y: panelY,
            w: panelWidth,
            h: panelHeight,
            keySize: keySize,
            keySpacing: keySpacing,
            padding: padding,
            titleHeight: titleHeight,
            cols: cols,
            rows: rows,
            bgColor: 'rgba(30, 30, 50, 0.7)',
            titleColor: 'rgba(200, 200, 255, 0.8)'
        };
        
        // 计算每个按键的位置 (第一行9个，第二行10个)
        for (var i = 0; i < this.vkbdKeys.length; i++) {
            var key = this.vkbdKeys[i];
            var row = i < 9 ? 0 : 1;  // 前9个在第一行，其余在第二行
            var col = i < 9 ? i : (i - 9);
            
            key.x = this.vkbdPanel.x + padding + col * (keySize + keySpacing);
            key.y = this.vkbdPanel.y + padding + titleHeight + row * (keySize + keySpacing);
            key.w = keySize;
            key.h = keySize;
        }
        
        console.log('[VirtualController] Virtual keyboard panel created (2 rows)');
    },
    
    // 延迟初始化 Canvas 2D 叠加层 (在游戏初始化后调用)
    lateInit: function() {
        if (this.mainCtx) return; // 已经初始化
        this.initCanvas2DOverlay();
    },
    
    // 初始化 Canvas 2D 叠加层
    initCanvas2DOverlay: function() {
        // 获取主 Canvas
        var mainCanvas = GameGlobal.__wxGameCanvas;
        if (!mainCanvas) {
            console.error('[VirtualController] Main canvas not found');
            return;
        }
        
        // 获取主 Canvas 的 2D 上下文
        this.mainCanvas = mainCanvas;
        this.mainCtx = mainCanvas.getContext('2d');
        if (!this.mainCtx) {
            console.error('[VirtualController] Failed to get 2D context from main canvas');
            return;
        }
        
        // 设置主 Canvas 的像素风格（禁用图像平滑）
        // 这对于像素艺术游戏非常重要，确保缩放时不会模糊
        this.mainCtx.imageSmoothingEnabled = false;
        this.mainCtx.webkitImageSmoothingEnabled = false;
        this.mainCtx.mozImageSmoothingEnabled = false;
        this.mainCtx.msImageSmoothingEnabled = false;
        this.mainCtx.oImageSmoothingEnabled = false;
        
        // 设置主 Canvas 的 CSS 样式（像素风格）
        if (mainCanvas.style) {
            mainCanvas.style.imageRendering = 'pixelated';
            mainCanvas.style.imageRendering = 'crisp-edges';
            mainCanvas.style.imageRendering = '-moz-crisp-edges';
            mainCanvas.style.imageRendering = '-webkit-optimize-contrast';
        }
        
        console.log('[VirtualController] Canvas 2D overlay initialized with pixel-perfect settings');
    },
    
    // 绘制虚拟按钮到离屏 Canvas
    render: function() {
        if (!this.visible || !this.ctx) return;
        
        // 优化：仅在需要时重绘
        if (!this.needsRedraw) return;
        
        var ctx = this.ctx;
        var startTime = Date.now();
        
        // 清除画布
        ctx.clearRect(0, 0, this.screenWidth, this.screenHeight);
        
        // 批量绘制游戏按钮（优化：使用预计算的参数）
        for (var name in this.buttons) {
            var btn = this.buttons[name];
            var params = this.buttonDrawParams[name];
            var isPressed = this.isButtonPressed(btn.id);
            
            // 编辑模式下显示高亮边框
            var buttonColor = btn.color;
            if (this.editMode) {
                buttonColor = 'rgba(255, 200, 100, 0.7)'; // 橙色高亮
            }
            
            // X按钮锁定状态下显示特殊颜色
            if (btn.id === this.BTN_X && this.xButtonLocked) {
                buttonColor = 'rgba(100, 100, 255, 0.7)'; // 蓝色高亮表示锁定
            }
            
            // 按钮背景
            ctx.beginPath();
            ctx.arc(params.centerX, params.centerY, params.radius, 0, Math.PI * 2);
            ctx.fillStyle = isPressed ? 'rgba(255, 255, 255, 0.5)' : buttonColor;
            ctx.fill();
            
            // 按钮边框
            ctx.strokeStyle = this.editMode ? 'rgba(255, 200, 100, 1.0)' : 'rgba(255, 255, 255, 0.8)';
            ctx.lineWidth = this.editMode ? 3 : 2;
            ctx.stroke();
            
            // 按钮文字
            ctx.fillStyle = isPressed ? '#000' : '#fff';
            ctx.font = 'bold ' + params.fontSize + 'px Arial';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText(btn.label, params.centerX, params.centerY);
        }
        
        // 绘制编辑按钮
        if (this.editButton) {
            var editBtn = this.editButton;
            var color = this.editMode ? editBtn.activeColor : editBtn.color;
            
            // 圆形按钮背景
            ctx.beginPath();
            ctx.arc(editBtn.centerX, editBtn.centerY, editBtn.radius, 0, Math.PI * 2);
            ctx.fillStyle = color;
            ctx.fill();
            
            // 边框
            ctx.strokeStyle = 'rgba(255, 255, 255, 0.8)';
            ctx.lineWidth = 2;
            ctx.stroke();
            
            // 文字
            ctx.fillStyle = '#fff';
            ctx.font = 'bold 9px Arial';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            var label = this.editMode ? editBtn.editLabel : editBtn.label;
            ctx.fillText(label, editBtn.centerX, editBtn.centerY);
        }
        
        // 绘制虚拟键盘按钮（仅在可见时）
        if (this.vkbdButton && this.vkbdButton.visible) {
            var kbBtn = this.vkbdButton;
            var color = this.vkbdVisible ? kbBtn.activeColor : kbBtn.color;
            
            // 圆形按钮背景
            ctx.beginPath();
            ctx.arc(kbBtn.centerX, kbBtn.centerY, kbBtn.radius, 0, Math.PI * 2);
            ctx.fillStyle = color;
            ctx.fill();
            
            // 边框
            ctx.strokeStyle = 'rgba(255, 255, 255, 0.8)';
            ctx.lineWidth = 2;
            ctx.stroke();
            
            // 文字
            ctx.fillStyle = '#fff';
            ctx.font = 'bold 9px Arial';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText(kbBtn.label, kbBtn.centerX, kbBtn.centerY);
        }
        
        // 绘制虚拟键盘面板
        if (this.vkbdVisible && this.vkbdPanel) {
            this.renderVkbdPanel(ctx);
        }
        
        this.needsRedraw = false;
        
        var elapsed = Date.now() - startTime;
        if (elapsed > 5) {
            console.log('[VirtualController] Render took', elapsed, 'ms');
        }
    },
    
    // 绘制虚拟键盘面板
    renderVkbdPanel: function(ctx) {
        var panel = this.vkbdPanel;
        
        // 面板背景
        ctx.fillStyle = panel.bgColor;
        ctx.fillRect(panel.x, panel.y, panel.w, panel.h);
        
        // 面板边框
        ctx.strokeStyle = 'rgba(200, 200, 255, 0.8)';
        ctx.lineWidth = 2;
        ctx.strokeRect(panel.x, panel.y, panel.w, panel.h);
        
        // 标题
        ctx.fillStyle = panel.titleColor;
        ctx.font = 'bold 10px Arial';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        ctx.fillText('KEYBOARD', panel.x + panel.w/2, panel.y + panel.titleHeight/2);
        
        // 绘制所有按键
        for (var i = 0; i < this.vkbdKeys.length; i++) {
            var key = this.vkbdKeys[i];
            var isPressed = this.vkbdPressedKey === key.code;
            
            // 按键背景
            ctx.fillStyle = isPressed ? 'rgba(255, 255, 255, 0.9)' : key.color;
            ctx.fillRect(key.x, key.y, key.w, key.h);
            
            // 按键边框
            ctx.strokeStyle = 'rgba(255, 255, 255, 0.8)';
            ctx.lineWidth = 1;
            ctx.strokeRect(key.x, key.y, key.w, key.h);
            
            // 按键文字
            ctx.fillStyle = isPressed ? '#000' : '#fff';
            ctx.font = 'bold ' + (key.label.length > 1 ? '8px' : '12px') + ' Arial';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText(key.label, key.x + key.w/2, key.y + key.h/2);
        }
    },
    
    // 将离屏 Canvas 通过 Canvas 2D 叠加到主画布
    renderOverlay: function() {
        if (!this.visible || !this.mainCtx){
            // console.log('[VirtualController] Overlay rendering skipped (not visible or mainCtx missing) this.visible=', this.visible, 'this.mainCtx=', this.mainCtx);
            return;
        }
        
        // 保存当前变换状态
        this.mainCtx.save();
        
        // 重置变换矩阵 (游戏渲染器可能改变了缩放)
        this.mainCtx.setTransform(1, 0, 0, 1, 0, 0);
        
        // 直接将离屏 Canvas 绘制到主 Canvas
        this.mainCtx.drawImage(this.offscreenCanvas, 0, 0);
        
        // 恢复变换状态
        this.mainCtx.restore();
    },
    
    // 优化：检查按钮是否被按下（使用缓存）
    isButtonPressed: function(btnId) {
        // X按钮的锁定状态优先
        if (btnId === this.BTN_X && this.xButtonLocked) {
            return true;
        }
        
        // 使用缓存
        if (this.buttonStateCache.hasOwnProperty(btnId)) {
            return this.buttonStateCache[btnId];
        }
        
        var pressed = false;
        for (var touchId in this.activeButtons) {
            if (this.activeButtons[touchId] === btnId) {
                pressed = true;
                break;
            }
        }
        
        this.buttonStateCache[btnId] = pressed;
        return pressed;
    },
    
    // 碰撞检测
    hitTest: function(x, y) {
        for (var name in this.buttons) {
            var btn = this.buttons[name];
            var params = this.buttonDrawParams[name];
            
            // 圆形碰撞检测（使用预计算的参数）
            var dx = x - params.centerX;
            var dy = y - params.centerY;
            if (dx*dx + dy*dy <= params.radius*params.radius) {
                return btn;
            }
        }
        return null;
    },
    
    // 发送按钮事件到 Rust
    sendButtonEvent: function(btnId, pressed) {
        
        if (!GameGlobal.wasm_bindgen) {
            console.error('[VirtualController] GameGlobal.wasm_bindgen not found!');
            return;
        }
        
        if (!GameGlobal.wasm_bindgen.on_button_event_cpu) {
            console.error('[VirtualController] on_button_event_cpu not found!');
            console.log('[VirtualController] Available methods:', Object.keys(GameGlobal.wasm_bindgen));
            return;
        }
        
        try {
            GameGlobal.wasm_bindgen.on_button_event_cpu(btnId, pressed);
        } catch (err) {
            console.error('[VirtualController] Error calling on_button_event_cpu:', err);
        }
    },
    
    // 触摸开始
    onTouchStart: function(e) {
        
        // 第一次触摸时恢复音频上下文
        if (!this.audioResumed) {
            this.audioResumed = true;
            this.resumeAudio();
        }
        
        for (var i = 0; i < e.touches.length; i++) {
            var touch = e.touches[i];
            var x = touch.clientX;
            var y = touch.clientY;
                        
            // 检查是否点击虚拟键盘按钮（仅在可见时）
            if (this.vkbdButton && this.vkbdButton.visible && this.hitTestRect(x, y, this.vkbdButton)) {
                this.toggleVkbdPanel();
                this.needsRedraw = true;
                return;
            }
            
            // 检查是否点击编辑按钮
            if (this.editButton && this.hitTestRect(x, y, this.editButton)) {
                this.editButtonClickCount++;
                
                // 点击5次后显示KB按钮
                if (this.editButtonClickCount >= 5 && this.vkbdButton && !this.vkbdButton.visible) {
                    this.vkbdButton.visible = true;
                }
                
                this.toggleEditMode();
                this.needsRedraw = true;
                return;
            }
            
            // 如果虚拟键盘可见，检查是否点击了键盘按键
            if (this.vkbdVisible) {
                var vkey = this.hitTestVkbdKey(x, y);
                if (vkey) {
                    this.vkbdPressedKey = vkey.code;
                    this.sendKeyEvent(vkey.code, true);
                    this.needsRedraw = true;
                    return;
                }
            }
            
            // 检查游戏按钮
            var btn = this.hitTest(x, y);
            
            if (btn) {
                if (this.editMode) {
                    // 编辑模式：开始拖动
                    this.dragInfo = {
                        touchId: touch.identifier,
                        button: btn,
                        startX: x,
                        startY: y,
                        btnStartX: btn.x,
                        btnStartY: btn.y
                    };
                } else {
                    // 游戏模式：发送按钮事件
                    if (!this.activeButtons[touch.identifier]) {
                        // X按钮特殊处理：切换锁定状态
                        if (btn.id === this.BTN_X) {
                            this.xButtonLocked = !this.xButtonLocked;
                            this.sendButtonEvent(btn.id, this.xButtonLocked);
                            this.buttonStateCache = {};
                            this.needsRedraw = true;
                        } else {
                            // 其他按钮正常处理
                            this.activeButtons[touch.identifier] = btn.id;
                            this.sendButtonEvent(btn.id, true);
                            this.buttonStateCache = {};
                            this.needsRedraw = true;
                        }
                    }
                }
            } else {
            }
        }
    },
    
    // 恢复音频上下文 (用户交互后调用)
    resumeAudio: function() {
        if (!GameGlobal.wasm_bindgen) {
            console.error('[VirtualController] GameGlobal.wasm_bindgen not found for resume_audio_cpu!');
            return;
        }
        
        if (GameGlobal.wasm_bindgen.resume_audio_cpu) {
            console.log('[VirtualController] Resuming audio context...');
            try {
                GameGlobal.wasm_bindgen.resume_audio_cpu();
                console.log('[VirtualController] Audio context resumed successfully');
            } catch (err) {
                console.error('[VirtualController] Error resuming audio:', err);
            }
        } else {
            console.warn('[VirtualController] resume_audio_cpu not found');
        }
    },
    
    // 触摸移动
    onTouchMove: function(e) {
        for (var i = 0; i < e.changedTouches.length; i++) {
            var touch = e.changedTouches[i];
            var x = touch.clientX;
            var y = touch.clientY;
            
            // 编辑模式：拖动按钮
            if (this.editMode && this.dragInfo && this.dragInfo.touchId === touch.identifier) {
                var dx = x - this.dragInfo.startX;
                var dy = y - this.dragInfo.startY;
                
                var newX = this.dragInfo.btnStartX + dx;
                var newY = this.dragInfo.btnStartY + dy;
                
                // 限制在屏幕范围内
                var btn = this.dragInfo.button;
                newX = Math.max(0, Math.min(this.screenWidth - btn.w, newX));
                newY = Math.max(0, Math.min(this.screenHeight - btn.h, newY));
                
                btn.x = newX;
                btn.y = newY;
                
                // 重新计算绘制参数
                var name = this.getButtonName(btn.id);
                if (name) {
                    this.buttonDrawParams[name].centerX = btn.x + btn.w/2;
                    this.buttonDrawParams[name].centerY = btn.y + btn.h/2;
                }
                
                this.needsRedraw = true;
                return;
            }
            
            // 游戏模式：检测按钮切换
            if (!this.editMode) {
                var oldBtnId = this.activeButtons[touch.identifier];
                var newBtn = this.hitTest(x, y);
                var newBtnId = newBtn ? newBtn.id : null;
                
                if (oldBtnId !== newBtnId) {
                    if (oldBtnId) {
                        this.sendButtonEvent(oldBtnId, false);
                    }
                    if (newBtnId) {
                        this.sendButtonEvent(newBtnId, true);
                        this.activeButtons[touch.identifier] = newBtnId;
                    } else {
                        delete this.activeButtons[touch.identifier];
                    }
                    
                    this.buttonStateCache = {};
                    this.needsRedraw = true;
                }
            }
        }
    },
    
    // 触摸结束
    onTouchEnd: function(e) {
        for (var i = 0; i < e.changedTouches.length; i++) {
            var touch = e.changedTouches[i];
            
            // 虚拟键盘按键释放
            if (this.vkbdPressedKey) {
                this.sendKeyEvent(this.vkbdPressedKey, false);
                this.vkbdPressedKey = null;
                this.needsRedraw = true;
                return;
            }
            
            // 编辑模式：结束拖动并保存
            if (this.editMode && this.dragInfo && this.dragInfo.touchId === touch.identifier) {
                this.saveButtonPositions();
                this.dragInfo = null;
                this.needsRedraw = true;
                return;
            }
            
            // 游戏模式：释放按钮
            var btnId = this.activeButtons[touch.identifier];
            if (btnId) {
                // X按钮在锁定状态下不释放
                if (btnId === this.BTN_X && this.xButtonLocked) {
                } else {
                    this.sendButtonEvent(btnId, false);
                    this.buttonStateCache = {};
                    this.needsRedraw = true;
                }
                delete this.activeButtons[touch.identifier];
            }
        }
    },
    
    // 切换虚拟键盘面板
    toggleVkbdPanel: function() {
        this.vkbdVisible = !this.vkbdVisible;
        this.needsRedraw = true;
    },
    
    // 虚拟键盘按键碰撞检测
    hitTestVkbdKey: function(x, y) {
        for (var i = 0; i < this.vkbdKeys.length; i++) {
            var key = this.vkbdKeys[i];
            if (x >= key.x && x <= key.x + key.w &&
                y >= key.y && y <= key.y + key.h) {
                return key;
            }
        }
        return null;
    },
    
    // 发送键盘事件到 Rust
    sendKeyEvent: function(code, pressed) {
        if (!GameGlobal.wasm_bindgen) {
            console.error('[VirtualController] GameGlobal.wasm_bindgen not found!');
            return;
        }
        
        if (!GameGlobal.wasm_bindgen.on_key_event_cpu) {
            console.error('[VirtualController] on_key_event_cpu not found!');
            return;
        }
        
        try {
            GameGlobal.wasm_bindgen.on_key_event_cpu(code, pressed);
        } catch (err) {
            console.error('[VirtualController] Error calling on_key_event_cpu:', err);
        }
    },
    
    // 切换编辑模式
    toggleEditMode: function() {
        this.editMode = !this.editMode;
        
        // 退出编辑模式时保存
        if (!this.editMode) {
            this.saveButtonPositions();
        }
        
        this.needsRedraw = true;
    },
    
    // 圆形碰撞检测（用于编辑按钮和KB按钮）
    hitTestCircle: function(x, y, circle) {
        var dx = x - circle.centerX;
        var dy = y - circle.centerY;
        return dx*dx + dy*dy <= circle.radius*circle.radius;
    },
    
    // 矩形碰撞检测（向后兼容，已废弃）
    hitTestRect: function(x, y, circle) {
        return this.hitTestCircle(x, y, circle);
    },
    
    // 根据按钮ID获取按钮名称
    getButtonName: function(btnId) {
        for (var name in this.buttons) {
            if (this.buttons[name].id === btnId) {
                return name;
            }
        }
        return null;
    },
    
    // 获取 Canvas (用于合成显示)
    getCanvas: function() {
        return this.offscreenCanvas;
    }
};

// ============================================================================
// Keyboard Controller - 微信小游戏 PC 端键盘支持
// https://developers.weixin.qq.com/minigame/dev/api/device/keyboard/wx.onKeyDown.html
// ============================================================================
var KeyboardController = {
    activeKeys: {},
    
    init: function() {
        console.log('[KeyboardController] Initializing...');
        
        var self = this;
        
        // 检查是否支持键盘事件 (仅 PC 微信客户端支持)
        if (typeof wx.onKeyDown === 'function' && typeof wx.onKeyUp === 'function') {
            wx.onKeyDown(function(res) {
                // console.log('[KeyboardController] Key down event:', res);
                self.onKeyDown(res);

                //隐藏触摸板
                if (GameGlobal.VirtualController) {
                    GameGlobal.VirtualController.visible = false;
                }
            });
            
            wx.onKeyUp(function(res) {
                // console.log('[KeyboardController] Key up event:', res);
                self.onKeyUp(res);
            });
            
            console.log('[KeyboardController] Keyboard events registered (PC mode)');
        } else {
            console.log('[KeyboardController] Keyboard events not supported (mobile mode)');
        }
    },
    
    // 发送键盘事件到 Rust
    sendKeyEvent: function(code, pressed) {
        if (!GameGlobal.wasm_bindgen) {
            console.error('[KeyboardController] GameGlobal.wasm_bindgen not found!');
            return;
        }
        
        if (!GameGlobal.wasm_bindgen.on_key_event_cpu) {
            console.error('[KeyboardController] on_key_event_cpu not found!');
            return;
        }
        
        try {
            GameGlobal.wasm_bindgen.on_key_event_cpu(code, pressed);
        } catch (err) {
            console.error('[KeyboardController] Error calling on_key_event_cpu:', err);
        }
    },
    
    // 键盘按下
    onKeyDown: function(res) {
        var code = res.code;  // 使用 code 属性 (如 "KeyA", "Space", "ControlLeft")
        
        // 防止重复触发 (按住不放时会持续触发)
        if (this.activeKeys[code]) {
            return;
        }
        
        this.activeKeys[code] = true;
        this.sendKeyEvent(code, true);
        
        // 调试日志 (前几次按键)
        if (Object.keys(this.activeKeys).length <= 5) {
            // console.log('[KeyboardController] keyDown:', code);
        }
    },
    
    // 键盘释放
    onKeyUp: function(res) {
        var code = res.code;
        
        delete this.activeKeys[code];
        this.sendKeyEvent(code, false);
    }
};

// ============================================================================
// Performance Monitor - 性能监控和GC管理
// ============================================================================
var PerformanceMonitor = {
    frameCount: 0,
    lastGCTime: 0,
    gcInterval: 5000, // 5秒触发一次GC
    
    init: function() {
        console.log('[PerformanceMonitor] Initialized');
        this.lastGCTime = Date.now();
    },
    
    onFrame: function() {
        this.frameCount++;
        
        // 定期触发GC（优化：避免内存积累）
        var now = Date.now();
        if (now - this.lastGCTime >= this.gcInterval) {
            this.triggerGC();
            this.lastGCTime = now;
        }
    },
    
    triggerGC: function() {
        if (typeof wx !== 'undefined' && wx.triggerGC) {
            wx.triggerGC();
            console.log('[PerformanceMonitor] GC triggered, frame:', this.frameCount);
        }
    }
};

// ============================================================================
// Main Game Entry
// ============================================================================

// Load WASM binding script first
require('./mario_wxgame_cpu.js');

// Start game after script is loaded
function startGame() {
    console.log('[game.js] Loading WASM...');
    
    // wasm_bindgen is now on GameGlobal
    var wasm_bindgen = GameGlobal.wasm_bindgen;
    
    if (!wasm_bindgen) {
        console.error('[game.js] wasm_bindgen not found on GameGlobal');
        return;
    }
    
    // Initialize WASM
    wasm_bindgen('mario_wxgame_cpu_bg.wasm')
        .then(function() {
            console.log('[game.js] WASM loaded successfully');
            
            // 设置目标帧率（微信小游戏优化）
            if (typeof wx !== 'undefined' && wx.setPreferredFramesPerSecond) {
                wx.setPreferredFramesPerSecond(60);
                console.log('[game.js] Set preferred FPS to 60');
            }
            
            // 初始化性能监控
            PerformanceMonitor.init();
            
            // Initialize virtual controller (touch)
            VirtualController.init();
            
            // Initialize keyboard controller (PC)
            KeyboardController.init();
            
            // 包装渲染函数以添加性能监控
            var originalRender = GameGlobal.renderVirtualController;
            GameGlobal.renderVirtualController = function() {
                PerformanceMonitor.onFrame();
                if (originalRender) {
                    originalRender();
                }
            };
            
            // Start game - CPU version
            if (wasm_bindgen.run_wxgame_cpu) {
                console.log('[game.js] Starting game (CPU version)...');
                wasm_bindgen.run_wxgame_cpu();
            } else {
                console.error('[game.js] run_wxgame_cpu not found');
                console.log('[game.js] Available:', Object.keys(wasm_bindgen));
            }
        })
        .catch(function(err) {
            console.error('[game.js] WASM load failed:', err);
        });
}

startGame();
