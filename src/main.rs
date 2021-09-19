#![windows_subsystem = "windows"]

use druid::{WindowDesc, AppLauncher};
use std::sync::{Arc, RwLock};
use crate::wc3proxy::Proxy;
use crate::ui::{AppEvents, build_root_widget, WINDOW_TITLE, AppState};

mod wc3proxy;
mod ui;


pub fn main() {


    // describe the main window
    let main_window = WindowDesc::new(build_root_widget)
        .title(WINDOW_TITLE)
        .resizable(false)
        .with_min_size((0.0, 0.0))
        .window_size((300.0, 200.0));

    // create the initial app state
    let initial_state = AppState::new(
        "9.9.9.9".to_string(),
        "6112".to_string(),
        None.into(),
        Arc::new(Vec::new()),
        Arc::new(RwLock::new(Proxy::new()))
    );

    // start the application
    AppLauncher::with_window(main_window)
        .delegate(AppEvents)
        .launch(initial_state)
        .expect("Failed to launch application");

}