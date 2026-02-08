mod config;
mod core;
mod filesystem;
mod thumbnail;
mod ui;

use gtk4::prelude::*;
use gtk4::Application;

const APP_ID: &str = "com.flear.diptych";

fn main() {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(ui::window::build);
    app.run();
}
