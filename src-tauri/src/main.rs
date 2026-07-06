// Standard Tauri v2 layout: keep main.rs a thin entry point, real setup lives in lib.rs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    waktu_lib::run();
}
