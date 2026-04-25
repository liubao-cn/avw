// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod http_client;
mod windsurf;

use tauri::Manager;

fn main() {
    let mut builder = tauri::Builder::default();

    // 桌面平台上 single-instance 必须最先注册
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let _ = argv;
            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.set_focus();
                let _ = main_window.unminimize();
            }
        }));
    }

    builder
        .invoke_handler(tauri::generate_handler![
            // Windsurf 账号管理命令（Windsurf-only 版本唯一保留的后端能力）
            windsurf::check_for_updates,
            windsurf::open_release_page,
            windsurf::reset_windsurf_machine_id,
            windsurf::windsurf_refresh_credits,
            windsurf::windsurf_switch_account,
            windsurf::windsurf_post_auth,
            windsurf::windsurf_add_account_by_auth1_token,
            windsurf::windsurf_add_account_by_session_token,
            windsurf::windsurf_get_analytics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
