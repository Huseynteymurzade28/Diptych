mod filesystem;
mod ui;

use gtk4::prelude::*;
use gtk4::Application;

const APP_ID: &str = "com.flear.diptych";

fn main() {
    // Create a new application
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    // Connect to "activate" signal using our UI module
    app.connect_activate(ui::window::build);

    // Run the application
    app.run();
}
