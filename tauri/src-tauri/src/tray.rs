// 托盘装配:图标 + 右键菜单 + 左键 toggle popover 窗口。
// 行为对齐 Sources/UsageBar/MenuBarController.swift:
//   - 左键托盘 → toggle popover(显示时定位到托盘附近并 refresh_now;已显示则收起)
//   - 右键托盘 → 菜单(立即刷新 / 开机自启[勾选态] / 退出)
//   - active_alert_count > 0 时切换告警图标 + tooltip
//
// 托盘图标用 `include_image!` 在编译期内嵌(路径相对 crate 根),
// 避免运行时按相对路径找文件(打包后 CWD 不确定)。

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

use tauri_plugin_autostart::ManagerExt;

use crate::state::AppState;

// 菜单项 id,on_menu_event 据此分发。
const MENU_REFRESH: &str = "tray_refresh";
const MENU_AUTOSTART: &str = "tray_autostart";
const MENU_QUIT: &str = "tray_quit";

// popover 窗口标签(与 tauri.conf.json 中一致)。
const POPOVER_LABEL: &str = "main";

// 编译期内嵌的两张托盘图标。
fn normal_icon() -> Image<'static> {
    tauri::include_image!("icons/tray.png")
}

fn alert_icon() -> Image<'static> {
    tauri::include_image!("icons/tray-alert.png")
}

// 构建托盘:图标 + 右键菜单 + 左右键事件。返回 TrayIcon 以便管理生命周期。
pub fn build_tray(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);

    let refresh = MenuItem::with_id(app, MENU_REFRESH, "立即刷新", true, None::<&str>)?;
    let autostart = CheckMenuItem::with_id(
        app,
        MENU_AUTOSTART,
        "开机自启",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&refresh, &autostart, &quit])?;

    // 把 autostart 菜单项 move 进事件闭包,切换后即时回写勾选态。
    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(normal_icon())
        .tooltip("订阅用量")
        .menu(&menu)
        // 左键不弹菜单,留给 toggle popover;菜单仅右键触发。
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| handle_menu_event(app, event, &autostart))
        .on_tray_icon_event(move |tray, event| handle_tray_event(tray, event))
        .build(app)?;

    Ok(tray)
}

// 右键菜单分发。autostart_item 用于切换后回写勾选态。
fn handle_menu_event(app: &AppHandle, event: MenuEvent, autostart_item: &CheckMenuItem<tauri::Wry>) {
    match event.id().as_ref() {
        MENU_REFRESH => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                state.refresh(&app).await;
            });
        }
        MENU_AUTOSTART => {
            let mgr = app.autolaunch();
            let now_enabled = mgr.is_enabled().unwrap_or(false);
            let result = if now_enabled {
                mgr.disable()
            } else {
                mgr.enable()
            };
            // 仅在操作成功时回写勾选态,失败则保持(避免与实际注册表状态不符)。
            if result.is_ok() {
                let _ = autostart_item.set_checked(!now_enabled);
            }
        }
        MENU_QUIT => {
            app.exit(0);
        }
        _ => {}
    }
}

// 托盘图标事件:左键 Up 时 toggle popover。
fn handle_tray_event(tray: &TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        position,
        ..
    } = event
    {
        toggle_popover(tray.app_handle(), position);
    }
}

// 显示/收起 popover 窗口。显示时定位到点击位置附近并触发一次刷新。
fn toggle_popover(app: &AppHandle, tray_pos: PhysicalPosition<f64>) {
    let Some(window) = app.get_webview_window(POPOVER_LABEL) else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }

    position_near_tray(&window, tray_pos);
    let _ = window.show();
    let _ = window.set_focus();

    // 显示即刷新一次(对齐 Swift togglePopover 的 store.refresh)。
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        state.refresh(&app).await;
    });
}

// 把窗口定位到托盘点击点附近:水平居中于点击点、贴屏幕底部上方(任务栏在底部时)。
// 落在所在显示器工作区内,避免越界。
fn position_near_tray(window: &WebviewWindow, tray_pos: PhysicalPosition<f64>) {
    let size = match window.outer_size() {
        Ok(s) => s,
        Err(_) => return,
    };
    let win_w = size.width as f64;
    let win_h = size.height as f64;

    // 默认:以托盘点击点为锚,窗口水平居中、底边略高于点击点。
    let mut x = tray_pos.x - win_w / 2.0;
    let mut y = tray_pos.y - win_h - 8.0;

    // 夹到所在显示器工作区内(work_area 已排除任务栏,留 8px 边距)。
    if let Ok(Some(monitor)) = window.monitor_from_point(tray_pos.x, tray_pos.y) {
        let area = monitor.work_area();
        let left = area.position.x as f64;
        let top = area.position.y as f64;
        let right = left + area.size.width as f64;
        let bottom = top + area.size.height as f64;
        let margin = 8.0;
        x = x.clamp(left + margin, (right - win_w - margin).max(left + margin));
        // y 若为负(点击点离顶部太近)则贴点击点下方。
        if y < top + margin {
            y = tray_pos.y + 8.0;
        }
        y = y.clamp(top + margin, (bottom - win_h - margin).max(top + margin));
    }

    let _ = window.set_position(PhysicalPosition::new(x, y));
}

// 根据是否有活跃告警切换托盘图标与 tooltip。由 lib.rs 在每次 usage-updated 后调用。
pub fn set_alert(app: &AppHandle, active_count: usize) {
    let Some(tray) = app.tray_by_id("main-tray") else {
        return;
    };
    if active_count > 0 {
        let _ = tray.set_icon(Some(alert_icon()));
        let _ = tray.set_tooltip(Some(format!("订阅用量:{} 个告警", active_count)));
    } else {
        let _ = tray.set_icon(Some(normal_icon()));
        let _ = tray.set_tooltip(Some("订阅用量"));
    }
}
