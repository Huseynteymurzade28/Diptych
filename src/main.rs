mod config;
mod core;
mod filesystem;
mod integration;
mod theme;
mod thumbnail;
mod ui;

use gtk4::prelude::*;

const APP_ID: &str = "com.flear.diptych";

fn main() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        // `diptych ~/Downloads`, and being the `inode/directory` handler.
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|app| {
        use integration::file_manager1;
        if app.flags().contains(gio::ApplicationFlags::IS_SERVICE) {
            // Started by D-Bus: stay up long enough for the call to arrive.
            app.set_inactivity_timeout(10_000);
        }
        if file_manager1::should_own(app) {
            let app = app.clone();
            // Owned for the rest of the process.
            let _owner = file_manager1::own(move |request| {
                for (dir, select) in integration::windows_for(&request.paths, request.reveal) {
                    ui::window::open(&app, Some(dir), select, Some(&request.startup_id));
                }
            });
        }
    });
    app.connect_activate(ui::window::build);
    app.connect_open(|app, files, _hint| {
        let paths: Vec<_> = files.iter().filter_map(|f| f.path()).collect();
        for (dir, select) in integration::windows_for(&paths, false) {
            ui::window::open(app, Some(dir), select, None);
        }
    });
    app.run();
}
