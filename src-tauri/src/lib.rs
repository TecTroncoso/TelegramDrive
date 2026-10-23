mod commands;
mod envfile;
mod error;
mod hash;
mod manifest;
mod state;
mod telegram;
mod turso;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            app.manage(state::AppState::new(data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_status,
            commands::save_credentials,
            commands::request_code,
            commands::submit_code,
            commands::submit_password,
            commands::logout,
            commands::save_turso,
            commands::list_files,
            commands::toggle_favorite,
            commands::trash_file,
            commands::restore_file,
            commands::purge_file,
            commands::upload_files,
            commands::sync_drive,
            commands::drive_info,
            commands::download_file,
            commands::reveal_path,
            commands::file_preview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
