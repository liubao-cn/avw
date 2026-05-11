// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auto_switch;
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
        .manage(auto_switch::AutoSwitchState::default())
        .setup(|app| {
            auto_switch::spawn_worker(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auto_switch::windsurf_auto_switch_get_status,
            auto_switch::windsurf_auto_switch_request_check,
            auto_switch::windsurf_auto_switch_update_snapshot,
            // Windsurf 账号管理命令（Windsurf-only 版本唯一保留的后端能力）
            windsurf::check_for_updates,
            windsurf::open_release_page,
            windsurf::reset_windsurf_machine_id,
            windsurf::windsurf_get_current_login,
            windsurf::windsurf_refresh_credits,
            windsurf::windsurf_switch_account,
            windsurf::windsurf_post_auth,
            windsurf::windsurf_add_account_by_auth1_token,
            windsurf::windsurf_add_account_by_session_token,
            windsurf::windsurf_get_analytics,
            // Windsurf 安装路径识别：支持自定义安装位置 / Windsurf - Next
            windsurf::detect_windsurf_install_path,
            windsurf::diagnose_windsurf_install_paths,
            windsurf::validate_windsurf_install_path,
            windsurf::windsurf_get_window_status,
            // 无感切号补丁
            windsurf::check_seamless_patch_status,
            windsurf::apply_seamless_patch,
            windsurf::restore_seamless_patch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
