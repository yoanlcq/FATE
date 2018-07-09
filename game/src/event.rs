use system::*;
use dmc::device::{Key, ButtonState};

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Quit,
    MouseMotion(f64, f64),
    CanvasResized(u32, u32),
    KeyboardKeyPressed(Key),
    KeyboardKeyReleased(Key),
    KeyboardTextChar(char),
}

impl Event {
    pub fn dispatch(&self, sys: &mut System, g: &mut G) {
        match *self {
            Event::Quit => sys.on_quit(g),
            Event::MouseMotion(x, y) => sys.on_mouse_motion(g, Vec2 { x, y }),
            Event::CanvasResized(w, h) => sys.on_canvas_resized(g, Extent2 { w, h }),
            Event::KeyboardKeyPressed(key) => sys.on_key(g, key, ButtonState::Down),
            Event::KeyboardKeyReleased(key) => sys.on_key(g, key, ButtonState::Up),
            Event::KeyboardTextChar(char) => sys.on_text_char(g, char),
        }
    }
}

