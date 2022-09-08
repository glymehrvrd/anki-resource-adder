#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use easy_error::{Error, ResultExt};
use std::{fs, io::BufWriter, path::Path};
use tauri::{AppHandle, Manager};

mod domains;
mod infra;

#[tauri::command]
async fn add_sound(
    apkg_path: &str,
    voice_dir: &str,
    out_apkg_path: &str,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let voice_dir_path = Path::new(voice_dir);
    let add = || {
        let mut col =
            domains::Collection::import(apkg_path, &app_handle).context("import failed")?;
        col.add_sound(voice_dir_path).context("add sound failed")?;
        col.export(
            BufWriter::new(fs::File::create(out_apkg_path).context("open file failed")?),
            &app_handle,
        )
        .context("export failed")?;

        app_handle.emit_all("onUpdateProgress", "处理完成...100%");
        return Ok(());
    };
    add().map_err(|x: Error| {
        app_handle.emit_all("onUpdateProgress", format!("处理失败: {}", x.to_string()));
        x.to_string()
    })
}

fn main() {
    env_logger::init();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![add_sound])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
