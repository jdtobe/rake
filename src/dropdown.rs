//! The drop-down window. This is OS glue around `tao` and `softbuffer`, so it is verified by
//! hand rather than unit tested; the placement maths lives in [`crate::geometry`].

use std::num::NonZeroU32;
use std::rc::Rc;

use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event_loop::EventLoopWindowTarget;
use tao::window::{Window, WindowBuilder, WindowId};

use crate::geometry::window_rect;

/// Placeholder background (0RGB) until a terminal renderer exists.
const PLACEHOLDER_COLOR: u32 = 0x001e_1e1e;

pub struct DropDown {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    height_percent: u8,
}

impl DropDown {
    /// Create the window hidden.
    pub fn new<T: 'static>(
        target: &EventLoopWindowTarget<T>,
        height_percent: u8,
    ) -> Result<Self, String> {
        let window = WindowBuilder::new()
            .with_title(env!("CARGO_PKG_NAME"))
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top(true)
            .with_visible(false)
            .build(target)
            .map_err(|e| format!("could not create window: {e}"))?;
        let window = Rc::new(window);
        let context = softbuffer::Context::new(window.clone())
            .map_err(|e| format!("could not create drawing context: {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| format!("could not create drawing surface: {e}"))?;
        Ok(DropDown {
            window,
            surface,
            height_percent,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    /// Place the window along the top of its monitor, then show and focus it.
    pub fn show(&self) {
        let monitor = self
            .window
            .current_monitor()
            .or_else(|| self.window.primary_monitor());
        if let Some(monitor) = monitor {
            let position = monitor.position();
            let size = monitor.size();
            let rect = window_rect(
                (position.x, position.y),
                (size.width, size.height),
                self.height_percent,
            );
            self.window
                .set_inner_size(PhysicalSize::new(rect.width, rect.height));
            self.window
                .set_outer_position(PhysicalPosition::new(rect.x, rect.y));
        }
        self.window.set_visible(true);
        self.window.set_focus();
        self.window.request_redraw();
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Fill the window with the placeholder color.
    pub fn paint(&mut self) {
        let size = self.window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return;
        };
        if self.surface.resize(width, height).is_err() {
            return;
        }
        if let Ok(mut buffer) = self.surface.buffer_mut() {
            buffer.fill(PLACEHOLDER_COLOR);
            let _ = buffer.present();
        }
    }
}
