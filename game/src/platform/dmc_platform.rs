use std::os::raw::c_void;
use std::collections::VecDeque;
use super::{Platform, Settings};
use event::Event;
use mouse_cursor::MouseCursor;
use dmc;
use fate::math::{Vec2, Extent2};

pub struct DmcPlatform {
    dmc: dmc::Context,
    window: dmc::Window,
    #[allow(dead_code)]
    gl_context: dmc::gl::GLContext,
    pending_events: VecDeque<Event>,
}

impl DmcPlatform {
    pub fn new(settings: &Settings) -> Self {
        let &Settings {
            ref title,
            canvas_size,
            ref gl_pixel_format_settings,
            ref gl_context_settings,
        } = settings;

        let dmc = dmc::Context::new().unwrap();

        let window = dmc.create_window(&dmc::WindowSettings {
            high_dpi: false,
            opengl: Some(&dmc::gl::GLDefaultPixelFormatChooser::from(gl_pixel_format_settings)),
        }).unwrap();

        window.set_size(canvas_size).unwrap();
        window.set_title(title).unwrap();

        let gl_context = window.create_gl_context(gl_context_settings).unwrap();
        window.make_gl_context_current(Some(&gl_context)).unwrap();

        if let Err(_) = window.gl_set_swap_interval(dmc::gl::GLSwapInterval::LateSwapTearing) {
            let _ = window.gl_set_swap_interval(dmc::gl::GLSwapInterval::VSync);
        }

        Self {
            dmc, window, gl_context,
            pending_events: VecDeque::with_capacity(8),
        }
    }
}

impl Platform for DmcPlatform {
    fn show_window(&mut self) {
        self.window.show().unwrap();
    }
    fn canvas_size(&self) -> Extent2<u32> {
        self.window.canvas_size().unwrap()
    }
    fn gl_swap_buffers(&mut self) {
        self.window.gl_swap_buffers().unwrap();
    }
    fn gl_get_proc_address(&self, proc: &str) -> *const c_void {
        self.gl_context.proc_address(proc)
    }
    fn poll_event(&mut self) -> Option<Event> {
        self.pump_events();
        self.pending_events.pop_front()
    }
    fn set_mouse_cursor(&mut self, mouse_cursor: &MouseCursor) {
        match *mouse_cursor {
            MouseCursor::System(c) => self.window.set_cursor(self.dmc.create_system_cursor(c).as_ref().unwrap()).unwrap(),
        }
    }
}

impl DmcPlatform {
    fn pump_events(&mut self) {
        while let Some(ev) = self.dmc.poll_event() {
            // debug!("DMC EVENT: {:?}", ev); // Tracing DMC events
            self.pump_dmc_event(ev);
        }
    }
    fn pump_dmc_event(&mut self, ev: dmc::Event) {
        let mut push = |e| self.pending_events.push_back(e);
        match ev {
            dmc::Event::Quit => push(Event::Quit),
            dmc::Event::WindowCloseRequested { .. } => push(Event::Quit),
            dmc::Event::WindowResized { size: Extent2 { w, h }, .. } => push(Event::CanvasResized(w, h)),
            dmc::Event::MouseEnter { .. } => push(Event::MouseEnter),
            dmc::Event::MouseLeave { .. } => push(Event::MouseLeave),
            dmc::Event::KeyboardFocusGained { .. } => push(Event::KeyboardFocusGained),
            dmc::Event::KeyboardFocusLost { .. } => push(Event::KeyboardFocusLost),
            dmc::Event::MouseButtonReleased { button, .. } => push(Event::MouseButtonReleased(button)),
            dmc::Event::MouseButtonPressed  { button, .. } => push(Event::MouseButtonPressed(button)),
            dmc::Event::MouseButtonReleasedRaw { button, .. } => push(Event::MouseButtonReleasedRaw(button)),
            dmc::Event::MouseButtonPressedRaw  { button, .. } => push(Event::MouseButtonPressedRaw(button)),
            dmc::Event::MouseMotion { position: Vec2 { x, y }, .. } => push(Event::MouseMotion(x as _, y as _)),
            dmc::Event::MouseMotionRaw { displacement: Vec2 { x, y }, .. } => push(Event::MouseMotionRaw(x as _, y as _)),
            dmc::Event::MouseScroll { scroll: Vec2 { x, y }, .. } => push(Event::MouseScroll(x as _, y as _)),
            dmc::Event::MouseScrollRaw { scroll: Vec2 { x, y }, .. } => push(Event::MouseScrollRaw(x as _, y as _)),
            dmc::Event::KeyboardKeyReleased { key, .. } => push(Event::KeyboardKeyReleased(key)),
            dmc::Event::KeyboardKeyPressed  { key,  is_repeat, .. } if !is_repeat => push(Event::KeyboardKeyPressed(key)),
            dmc::Event::KeyboardKeyReleasedRaw { key, .. } => push(Event::KeyboardKeyReleasedRaw(key)),
            dmc::Event::KeyboardKeyPressedRaw  { key, .. } => push(Event::KeyboardKeyPressedRaw(key)),
            dmc::Event::KeyboardTextChar    { char, .. } => push(Event::KeyboardTextChar(char)),
            dmc::Event::KeyboardTextString  { ref text, .. } => {
                for char in text.chars() {
                    push(Event::KeyboardTextChar(char));
                }
            },
            _ => (),
        }
    }
}
