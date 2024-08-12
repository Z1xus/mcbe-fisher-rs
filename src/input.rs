use rdev::{simulate, Button, EventType};
use std::{thread, time::Duration};

pub enum Key {
    MouseRight,
}

pub fn send_key(key: Key) {
    let event = match key {
        Key::MouseRight => EventType::ButtonPress(Button::Right),
    };

    if let Err(e) = simulate(&event) {
        println!("failed to send key event: {:?}", e);
    }

    thread::sleep(Duration::from_millis(50));

    let release = match key {
        Key::MouseRight => EventType::ButtonRelease(Button::Right),
    };

    if let Err(e) = simulate(&release) {
        println!("failed to send key release event: {:?}", e);
    }
}
