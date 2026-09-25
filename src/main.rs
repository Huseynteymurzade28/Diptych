mod config;
mod core;
mod filesystem;
mod theme;
mod thumbnail;
mod ui;

use gtk4::prelude::*;

const APP_ID: &str = "com.flear.diptych";

fn main() {
    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_activate(ui::window::build);
    app.run();
}
