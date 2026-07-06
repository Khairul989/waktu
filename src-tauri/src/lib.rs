pub mod api;
pub mod cache;
pub mod commands;
pub mod config;
pub mod location;
pub mod models;
pub mod prayer;
pub mod tray;

use tauri::Manager;
// tauri-nspanel v2.0.1 only exposes panel config through the deprecated `cocoa`
// re-export; there is no non-deprecated entry point for these constants yet.
#[allow(deprecated)]
use tauri_nspanel::{
    cocoa::appkit::{NSMainMenuWindowLevel, NSWindowCollectionBehavior},
    panel_delegate, ManagerExt, WebviewWindowExt,
};

// NSWindowStyleMaskNonActivatingPanel: lets the panel become key without
// activating the app, so it never triggers the focus-stealing/blur race a
// regular window has when another app is frontmost.
const NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL: i32 = 1 << 7;

// tauri-nspanel v2.0.1's own public API surface (cocoa types, panel_delegate!
// macro expansion) is deprecated/lint-noisy upstream; there is no alternative
// entry point, so silence it here the same way the crate does internally.
#[allow(deprecated, unexpected_cfgs)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_nspanel::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::list_zones,
            commands::set_zone,
            commands::clear_zone,
            commands::refresh,
        ])
        .setup(|app| {
            // Menubar-only: no dock icon, no app-switcher entry.
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::build(app.handle())?;
            tray::spawn_updater(app.handle().clone());

            // Convert the popover window into a non-activating NSPanel: it floats
            // above every app/Space and can hold key focus without stealing app
            // activation, which is what a plain window can't do in the background.
            if let Some(window) = app.get_webview_window("main") {
                let panel = window.to_panel()?;
                panel.set_level(NSMainMenuWindowLevel + 1);
                panel.set_collection_behaviour(
                    NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                        | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary,
                );
                panel.set_style_mask(NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL);

                // Popover behaves like a real menubar dropdown: losing key focus
                // (click-away) closes it.
                let delegate = panel_delegate!(WaktuPanelDelegate { window_did_resign_key });
                let handle = app.handle().clone();
                delegate.set_listener(Box::new(move |name: String| {
                    if name == "window_did_resign_key" {
                        if let Ok(panel) = handle.get_webview_panel("main") {
                            panel.order_out(None);
                        }
                    }
                }));
                panel.set_delegate(delegate);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
