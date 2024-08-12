#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fisher;
mod input;
mod memory;
mod window;

use window::FisherUi;

fn main() {
    let app = FisherUi::new();
    if let Err(e) = app.run() {
        eprintln!("error running app: {}", e);
    }
}
