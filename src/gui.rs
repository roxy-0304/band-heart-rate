use std::cell::Cell;
use std::rc::Rc;
use tokio::sync::watch;

use crate::types::HeartRateReading;

// Include the Slint UI generated at build time
slint::include_modules!();

const MENU_QUIT: &str = "quit";
const ZONES: &[(f64, f64, &str, &str)] = &[
    (0.0, 0.6, "热身", "#4FC3F7"),
    (0.6, 0.7, "燃脂", "#66BB6A"),
    (0.7, 0.8, "有氧", "#FFA726"),
    (0.8, 1.0, "极限", "#EF5350"),
];

fn get_zone(hr: u16) -> (&'static str, &'static str) {
    if hr == 0 {
        return ("--", "#4A4F58");
    }
    let pct = hr as f64 / 190.0;
    for &(min, max, label, color) in ZONES {
        if pct >= min && pct < max {
            return (label, color);
        }
    }
    ("极限", "#EF5350")
}

/// Blend zone color at 12% opacity over background #0a0e16
fn zone_bg_color(hex: &str) -> slint::Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64;
    let bg_r = 10.0f64;
    let bg_g = 14.0f64;
    let bg_b = 22.0f64;
    let alpha = 0.12f64;
    let out_r = (bg_r * (1.0 - alpha) + r * alpha) as u8;
    let out_g = (bg_g * (1.0 - alpha) + g * alpha) as u8;
    let out_b = (bg_b * (1.0 - alpha) + b * alpha) as u8;
    slint::Color::from_rgb_u8(out_r, out_g, out_b)
}

fn parse_color(hex: &str) -> slint::Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    slint::Color::from_rgb_u8(r, g, b)
}

/// Load embedded icon PNG as tray_icon::Icon
fn load_tray_icon() -> anyhow::Result<tray_icon::Icon> {
    let rgba = image::load_from_memory(include_bytes!("../icons/icon.png"))
        .map_err(|e| anyhow::anyhow!("Failed to load tray icon: {e}"))?
        .into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(tray_icon::Icon::from_rgba(rgba.into_raw(), width, height)?)
}

/// Create a hidden popup window for TrackPopupMenu ownership.
fn create_hidden_window() -> windows_sys::Win32::Foundation::HWND {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;

    let class_name: Vec<u16> = "BandHeartRateTray\0".encode_utf16().collect();
    unsafe {
        let h_instance = GetModuleHandleW(std::ptr::null());
        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: h_instance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        RegisterClassExW(&wnd_class);
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            std::ptr::null(),
            WS_POPUP,
            0, 0, 0, 0,
            0 as windows_sys::Win32::Foundation::HWND,
            std::ptr::null_mut(),
            h_instance,
            std::ptr::null(),
        )
    }
}

/// Show the tray context menu at the current cursor position.
/// Uses TPM_RETURNCMD so it blocks and returns the selected item ID (0 if cancelled).
/// Follows Microsoft's recommended pattern (KB Q135788):
/// SetForegroundWindow → TrackPopupMenu → PostMessage(WM_NULL)
/// This ensures the menu auto-dismisses when clicking outside.
fn show_tray_context_menu(hwnd: windows_sys::Win32::Foundation::HWND, menu: &muda::Menu) -> u32 {
    use muda::ContextMenu;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        TrackPopupMenu, GetCursorPos, SetForegroundWindow, PostMessageW,
        TPM_RIGHTBUTTON, TPM_BOTTOMALIGN, TPM_RETURNCMD, WM_NULL,
    };
    unsafe {
        // Required before TrackPopupMenu per Microsoft docs
        SetForegroundWindow(hwnd);

        let mut point: windows_sys::Win32::Foundation::POINT = std::mem::zeroed();
        GetCursorPos(&mut point);
        let result = TrackPopupMenu(
            menu.hpopupmenu() as *mut _,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null_mut(),
        ) as u32;

        // Required after TrackPopupMenu per Microsoft docs
        // Ensures menu is dismissed when focus moves away
        PostMessageW(hwnd, WM_NULL, 0, 0);

        result
    }
}

/// Pump Windows messages so tray-icon's WndProc can receive events
fn pump_windows_messages() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{PeekMessageW, TranslateMessage, DispatchMessageW, MSG};
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, 1) } != 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Run the Slint GUI, consuming heart rate data from the watch channel.
pub fn run(rx: watch::Receiver<HeartRateReading>) -> anyhow::Result<()> {
    let window = MainWindow::new()?;

    // Create a hidden window for popup menu ownership
    let menu_hwnd = create_hidden_window();

    // Create the popup menu
    let menu = muda::Menu::new();
    let quit_item = muda::MenuItem::with_id(MENU_QUIT, "退出", true, None::<muda::accelerator::Accelerator>);
    let _ = menu.append(&quit_item);

    // Set up system tray — NO .with_menu(), handle right-click manually
    let icon = load_tray_icon().ok();
    let _tray = icon.and_then(|icon| {
        tray_icon::TrayIconBuilder::new()
            .with_tooltip("Band Heart Rate Monitor")
            .with_icon(icon)
            .build()
            .ok()
    });

    // Intercept window close: hide and let window.run() return
    let w = window.as_weak();
    window.window().on_close_requested(move || {
        if let Some(win) = w.upgrade() {
            let _ = win.window().hide();
        }
        slint::CloseRequestResponse::HideWindow
    });

    // Stats state
    let stats_min = Rc::new(Cell::new(0u16));
    let stats_max = Rc::new(Cell::new(0u16));
    let stats_sum = Rc::new(Cell::new(0u32));
    let stats_count = Rc::new(Cell::new(0u32));

    // Reset button handler
    {
        let window_weak = window.as_weak();
        let sm = stats_min.clone();
        let sx = stats_max.clone();
        let ss = stats_sum.clone();
        let sc = stats_count.clone();
        window.on_reset_stats(move || {
            sm.set(0);
            sx.set(0);
            ss.set(0);
            sc.set(0);
            if let Some(w) = window_weak.upgrade() {
                w.set_stat_min("--".into());
                w.set_stat_max("--".into());
                w.set_stat_avg("--".into());
                w.set_bpm_text("--".into());
                w.set_zone_label("--".into());
                w.set_zone_color(parse_color("#4A4F58"));
                w.set_zone_bg_color(zone_bg_color("#4A4F58"));
                w.set_heart_beating(false);
            }
        });
    }

    // Timer for heart rate data
    let window_weak = window.as_weak();
    let rx = rx.clone();

    let _timer = slint::Timer::default();
    _timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            // Always borrow the latest value from the watch channel
            let data = rx.borrow().clone();

            let Some(w) = window_weak.upgrade() else {
                return;
            };

            // Status
            if let Some(ref err) = data.error {
                w.set_status_text(format!("错误: {err}").into());
                w.set_status_color(parse_color("#EF4444"));
            } else if data.connected {
                w.set_status_text("已连接".into());
                w.set_status_color(parse_color("#4ADE80"));
            } else if data.scanning {
                w.set_status_text("扫描中...".into());
                w.set_status_color(parse_color("#FBBF24"));
            } else {
                w.set_status_text("断开连接".into());
                w.set_status_color(parse_color("#EF4444"));
            }

            // Heart rate
            if data.connected && data.heart_rate > 0 {
                w.set_bpm_text(data.heart_rate.to_string().into());
                w.set_heart_beating(true);

                // Zone
                let (zone_label, zone_color) = get_zone(data.heart_rate);
                w.set_zone_label(zone_label.into());
                w.set_zone_color(parse_color(zone_color));
                w.set_zone_bg_color(zone_bg_color(zone_color));

                // Stats
                let hr = data.heart_rate;
                if stats_count.get() == 0 {
                    stats_min.set(hr);
                    stats_max.set(hr);
                    stats_sum.set(hr as u32);
                    stats_count.set(1);
                } else {
                    if hr < stats_min.get() { stats_min.set(hr); }
                    if hr > stats_max.get() { stats_max.set(hr); }
                    stats_sum.set(stats_sum.get() + hr as u32);
                    stats_count.set(stats_count.get() + 1);
                }

                w.set_stat_min(stats_min.get().to_string().into());
                w.set_stat_max(stats_max.get().to_string().into());
                let avg = stats_sum.get() / stats_count.get();
                w.set_stat_avg(avg.to_string().into());
            } else if !data.connected && !data.scanning {
                w.set_bpm_text("--".into());
                w.set_zone_label("--".into());
                w.set_zone_color(parse_color("#4A4F58"));
                w.set_zone_bg_color(zone_bg_color("#4A4F58"));
                w.set_heart_beating(false);
            }
        },
    );

    // Main loop
    let mut should_quit = false;

    while !should_quit {
        window.run()?;

        // window.run() returned — window was hidden via HideWindow
        // Wait briefly and flush any residual events
        std::thread::sleep(std::time::Duration::from_millis(300));
        pump_windows_messages();
        while tray_icon::TrayIconEvent::receiver().try_recv().is_ok() {}
        while muda::MenuEvent::receiver().try_recv().is_ok() {}

        // Poll tray events
        loop {
            pump_windows_messages();

            let mut want_show = false;
            while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
                match event {
                    // Left single-click → show window
                    tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } => {
                        want_show = true;
                    }
                    // Right single-click → show context menu manually
                    tray_icon::TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Right,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } => {
                        let selected = show_tray_context_menu(menu_hwnd, &menu);
                        // TrackPopupMenu with TPM_RETURNCMD blocks until menu is closed.
                        // It returns 0 if user clicks outside (loses focus) or presses Esc.
                        // It returns the menu item ID if user selects one.
                        // Drain any residual tray events generated during menu display.
                        while tray_icon::TrayIconEvent::receiver().try_recv().is_ok() {}
                        if selected > 0 {
                            should_quit = true;
                        }
                    }
                    _ => {}
                }
            }

            // Also check menu events from Slint event loop phase
            while let Ok(event) = muda::MenuEvent::receiver().try_recv() {
                if event.id().0.as_str() == MENU_QUIT {
                    should_quit = true;
                    break;
                }
            }

            if should_quit || want_show {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if !should_quit {
            let _ = window.window().show();
        }
    }

    Ok(())
}