#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use easy_error::{Error, ResultExt};
use std::{fs, io::BufWriter};
use tauri::Manager;

mod domains;
mod infra;

#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String,
}

fn add_sound_inner(
    apkg_path: &str,
    voice_dir: &str,
    out_apkg_path: &str,
    app_handle: tauri::AppHandle,
) -> Result<(), Error> {
    let mut col = domains::Collection::import(apkg_path).context("import failed")?;
    col.add_sound().context("add sound failed")?;
    col.export(BufWriter::new(
        fs::File::create("/Users/jasonjsyuan/Downloads/out.apkg").context("open file failed")?,
    ))
    .context("export failed")?;

    app_handle.emit_all("onUpdateProgress", "处理完成...100%");
    return Ok(());
}

#[tauri::command]
async fn add_sound(
    apkg_path: &str,
    voice_dir: &str,
    out_apkg_path: &str,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    match add_sound_inner(apkg_path, voice_dir, out_apkg_path, app_handle) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn main() {
    env_logger::init();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![add_sound])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
