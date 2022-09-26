#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{path::Path, sync::RwLock};

use tauri::State;

use blossom_core::Repo;

fn main() {
    tauri::Builder::default()
        .manage(RwLock::new(Repo::new()))
        .invoke_handler(tauri::generate_handler![
            get_repo_tree,
            add_protobuf_descriptor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Returns `blossom_types::repo::RepoView` but JSON encoded
#[tauri::command]
fn get_repo_tree(repo: State<RwLock<Repo>>) -> Result<String, String> {
    let repo_view = repo.read().expect("previous holder panicked").view();
    serde_json::to_string(&repo_view).map_err(|err| err.to_string())
}

#[tauri::command]
fn add_protobuf_descriptor(path: &Path, repo: State<RwLock<Repo>>) -> Result<(), String> {
    let mut repo = repo.write().expect("previous holder panicked");
    repo.add_descriptor(path).map_err(|err| err.to_string())
}
