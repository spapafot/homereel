mod auth;
mod commands;
mod config;
mod db;
mod drive;
mod error;
mod ffmpeg;
mod former_name;
mod library;
mod protocol;
mod recorded_date;
mod state;
mod titles;

use tauri::Manager;

use crate::protocol::{person_photos, thumbs};
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let context = tauri::generate_context!();
    if let Err(error) = former_name::move_data(&context.config().identifier, config::DATABASE_FILE)
    {
        // Starting anyway would create an empty library and strand the old one.
        former_name::report_move_failure(&error);
        std::process::exit(1);
    }

    let log_level = if cfg!(debug_assertions) {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };

    tauri::Builder::default()
        // Must be registered first so a second launch focuses the running window instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log_level)
                .build(),
        )
        // Thumbnails are served as `thumb://localhost/<videoId>` (http://thumb.localhost on
        // Windows), so <img> tags can load them without exposing tokens to the page.
        .register_asynchronous_uri_scheme_protocol("thumb", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let file_id = request.uri().path().trim_start_matches('/').to_owned();
            tauri::async_runtime::spawn(async move {
                let response = match app.try_state::<AppState>() {
                    Some(state) => thumbs::respond(&state.thumbnails, &file_id).await,
                    None => thumbs::unavailable(),
                };
                responder.respond(response);
            });
        })
        .register_asynchronous_uri_scheme_protocol("personphoto", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let photo_id = request.uri().path().trim_start_matches('/').to_owned();
            tauri::async_runtime::spawn(async move {
                let response = match app.try_state::<AppState>() {
                    Some(state) => person_photos::respond(&state.db, &photo_id),
                    None => thumbs::unavailable(),
                };
                responder.respond(response);
            });
        })
        .setup(|app| {
            let cache_dir = app.path().app_cache_dir()?;
            let data_dir = app.path().app_data_dir()?;
            let state =
                tauri::async_runtime::block_on(AppState::initialize(&cache_dir, &data_dir))?;
            let has_sources = state.has_sources().unwrap_or(false);
            app.manage(state);
            if has_sources {
                app.state::<AppState>().date_scan.start(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::auth_status,
            commands::auth::auth_connect,
            commands::auth::auth_cancel,
            commands::auth::auth_disconnect,
            commands::library::videos_list,
            commands::library::shorts_list,
            commands::library::video_get,
            commands::library::library_list,
            commands::library::folders_list,
            commands::playback::playback_source,
            commands::playback::playback_open_external,
            commands::playback::video_show_in_folder,
            commands::playback::playback_open_hevc_store,
            commands::sources::library_sources_get,
            commands::sources::library_local_folder_add,
            commands::sources::library_local_folder_remove,
            commands::sources::library_drive_folder_set,
            commands::sources::drive_folders_list,
            commands::user_state::video_set_favorite,
            commands::user_state::video_set_watch_later,
            commands::user_state::video_set_custom_title,
            commands::user_state::video_record_progress,
            commands::user_state::history_remove,
            commands::user_state::history_clear,
            commands::playlists::playlists_list,
            commands::playlists::playlist_get,
            commands::playlists::playlist_create,
            commands::playlists::playlist_rename,
            commands::playlists::playlist_delete,
            commands::playlists::playlist_add_video,
            commands::playlists::playlist_remove_video,
            commands::playlists::playlist_reorder,
            commands::playlists::playlists_for_video,
            commands::titles::gemini_settings_get,
            commands::titles::gemini_key_set,
            commands::titles::gemini_key_remove,
            commands::titles::gemini_models_list,
            commands::titles::gemini_model_set,
            commands::titles::gemini_instructions_set,
            commands::titles::titles_status,
            commands::titles::titles_start,
            commands::titles::titles_pause,
            commands::titles::title_regenerate,
            commands::people::people_list,
            commands::people::person_save,
            commands::people::person_delete,
            commands::people::video_people,
            commands::people::video_frame_capture,
            commands::thumbnails::thumbnail_regenerate,
        ])
        .run(context)
        .expect("error while running Homereel");
}
