// 测试 Windows Joystick API (mmsystem)
// 运行: cargo run --example test_joystick --features gdi-backend
//
// 这个程序使用 Windows 多媒体 API 读取手柄输入
// 可以检测 joy.cpl 中显示的任何手柄

#[cfg(target_os = "windows")]
fn main() {
    use std::mem::MaybeUninit;
    
    // Windows Joystick API 常量
    const JOYSTICKID1: u32 = 0;
    const JOYSTICKID2: u32 = 1;
    const JOYERR_NOERROR: u32 = 0;
    const JOY_RETURNALL: u32 = 0x00FF;
    
    // JOYINFOEX 结构体
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    struct JOYINFOEX {
        dw_size: u32,
        dw_flags: u32,
        dw_xpos: u32,
        dw_ypos: u32,
        dw_zpos: u32,
        dw_rpos: u32,
        dw_upos: u32,
        dw_vpos: u32,
        dw_buttons: u32,
        dw_button_number: u32,
        dw_pov: u32,
        dw_reserved1: u32,
        dw_reserved2: u32,
    }
    
    // JOYCAPS 结构体 (简化版)
    #[repr(C)]
    #[derive(Debug)]
    struct JOYCAPSW {
        w_mid: u16,
        w_pid: u16,
        sz_pname: [u16; 32],
        w_xmin: u32,
        w_xmax: u32,
        w_ymin: u32,
        w_ymax: u32,
        w_zmin: u32,
        w_zmax: u32,
        w_num_buttons: u32,
        w_period_min: u32,
        w_period_max: u32,
        w_rmin: u32,
        w_rmax: u32,
        w_umin: u32,
        w_umax: u32,
        w_vmin: u32,
        w_vmax: u32,
        w_caps: u32,
        w_max_axes: u32,
        w_num_axes: u32,
        w_max_buttons: u32,
        sz_reg_key: [u16; 32],
        sz_oem_vx_d: [u16; 260],
    }
    
    #[link(name = "winmm")]
    unsafe extern "system" {
        fn joyGetNumDevs() -> u32;
        fn joyGetPosEx(joy_id: u32, pji: *mut JOYINFOEX) -> u32;
        fn joyGetDevCapsW(joy_id: u32, pjc: *mut JOYCAPSW, cbjc: u32) -> u32;
    }
    
    println!("=== Windows Joystick API 测试 ===\n");
    
    // 获取系统支持的手柄数量
    let num_devs = unsafe { joyGetNumDevs() };
    println!("系统支持最多 {} 个手柄\n", num_devs);
    
    // 检测已连接的手柄
    let mut found_joystick = None;
    
    for joy_id in 0..num_devs.min(16) {
        let mut caps: MaybeUninit<JOYCAPSW> = MaybeUninit::uninit();
        let result = unsafe { 
            joyGetDevCapsW(joy_id, caps.as_mut_ptr(), std::mem::size_of::<JOYCAPSW>() as u32) 
        };
        
        if result == JOYERR_NOERROR {
            let caps = unsafe { caps.assume_init() };
            let name: String = caps.sz_pname
                .iter()
                .take_while(|&&c| c != 0)
                .map(|&c| char::from_u32(c as u32).unwrap_or('?'))
                .collect();
            
            println!("[手柄 {}] {}", joy_id, name);
            println!("  - 按钮数: {}", caps.w_num_buttons);
            println!("  - 轴数: {}", caps.w_num_axes);
            println!("  - X范围: {} - {}", caps.w_xmin, caps.w_xmax);
            println!("  - Y范围: {} - {}", caps.w_ymin, caps.w_ymax);
            println!();
            
            if found_joystick.is_none() {
                found_joystick = Some((joy_id, caps));
            }
        }
    }
    
    let Some((joy_id, caps)) = found_joystick else {
        println!("未检测到手柄！请确保手柄已连接。");
        return;
    };
    
    println!("开始读取手柄 {} 的输入（按 Ctrl+C 退出）...\n", joy_id);
    
    // 计算轴的中心值和阈值
    let x_center = (caps.w_xmin + caps.w_xmax) / 2;
    let y_center = (caps.w_ymin + caps.w_ymax) / 2;
    let x_threshold = (caps.w_xmax - caps.w_xmin) / 4;
    let y_threshold = (caps.w_ymax - caps.w_ymin) / 4;
    
    let mut last_buttons = 0u32;
    let mut last_direction = String::new();
    
    loop {
        let mut info = JOYINFOEX {
            dw_size: std::mem::size_of::<JOYINFOEX>() as u32,
            dw_flags: JOY_RETURNALL,
            dw_xpos: 0,
            dw_ypos: 0,
            dw_zpos: 0,
            dw_rpos: 0,
            dw_upos: 0,
            dw_vpos: 0,
            dw_buttons: 0,
            dw_button_number: 0,
            dw_pov: 0,
            dw_reserved1: 0,
            dw_reserved2: 0,
        };
        
        let result = unsafe { joyGetPosEx(joy_id, &mut info) };
        
        if result == JOYERR_NOERROR {
            // 检测按钮变化
            if info.dw_buttons != last_buttons {
                let pressed: Vec<u32> = (0..32)
                    .filter(|i| (info.dw_buttons & (1 << i)) != 0)
                    .collect();
                if !pressed.is_empty() {
                    println!("按钮按下: {:?}", pressed);
                }
                last_buttons = info.dw_buttons;
            }
            
            // 检测方向
            let mut direction = String::new();
            if info.dw_xpos < x_center - x_threshold { direction.push_str("左 "); }
            if info.dw_xpos > x_center + x_threshold { direction.push_str("右 "); }
            if info.dw_ypos < y_center - y_threshold { direction.push_str("上 "); }
            if info.dw_ypos > y_center + y_threshold { direction.push_str("下 "); }
            
            if direction != last_direction {
                if !direction.is_empty() {
                    println!("方向: {}", direction.trim());
                }
                last_direction = direction;
            }
        }
        
        std::thread::sleep(std::time::Duration::from_millis(16)); // 约 60 FPS
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("此示例仅支持 Windows 平台");
}
