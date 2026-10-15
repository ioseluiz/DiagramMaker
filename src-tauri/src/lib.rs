pub mod commands;
pub mod error;
pub mod io;
pub mod model;
pub mod render;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::document_new,
            commands::document_open,
            commands::document_save,
            commands::document_validate,
            commands::export_svg,
            commands::export_to_file,
            commands::recent_list,
            commands::recent_remember,
            commands::recent_forget,
            commands::autosave_write,
            commands::autosave_read,
            commands::autosave_clear,
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar la aplicacion Tauri");
}
