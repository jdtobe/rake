//! The drop-down window. This is OS glue around `tao` and `softbuffer`, so it is verified by
//! hand rather than unit tested; the placement maths lives in [`crate::geometry`] and the
//! slide maths in [`crate::animation`].

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event_loop::EventLoopWindowTarget;
use tao::window::{Window, WindowBuilder, WindowId};

use crate::animation::Slide;
use crate::config::Config;
use crate::geometry::{window_rect, Rect};

/// Placeholder background (0RGB) until a terminal renderer exists.
const PLACEHOLDER_COLOR: u32 = 0x001e_1e1e;

pub struct DropDown {
    window: Rc<Window>,
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    width_percent: u8,
    height_percent: u8,
    animation: Duration,
    /// Where the window rests while shown. Also where it is parked while hidden, so the next
    /// `show` finds the same monitor.
    home: Option<Rect>,
    /// Current top edge of the window, in physical pixels.
    y: i32,
    visible: bool,
    /// The running slide is a slide out, so the window is hidden when it finishes.
    hiding: bool,
    slide: Option<Slide>,
}

impl DropDown {
    /// Create the window hidden.
    pub fn new<T: 'static>(
        target: &EventLoopWindowTarget<T>,
        config: &Config,
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
            width_percent: config.width_percent,
            height_percent: config.height_percent,
            animation: Duration::from_millis(config.animation_ms.into()),
            home: None,
            y: 0,
            visible: false,
            hiding: false,
            slide: None,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    /// True while a slide is running, so the event loop should keep waking up.
    pub fn is_animating(&self) -> bool {
        self.slide.is_some()
    }

    /// Slide the window in from above the top of its monitor, then focus it. If it is partway
    /// out, it reverses from where it is.
    pub fn show(&mut self, now: Instant) {
        let monitor = self
            .window
            .current_monitor()
            .or_else(|| self.window.primary_monitor());
        let Some(monitor) = monitor else {
            self.window.set_visible(true);
            self.window.set_focus();
            return;
        };
        let position = monitor.position();
        let size = monitor.size();
        let home = window_rect(
            (position.x, position.y),
            (size.width, size.height),
            self.width_percent,
            self.height_percent,
        );
        self.home = Some(home);
        self.window
            .set_inner_size(PhysicalSize::new(home.width, home.height));

        if !self.visible {
            self.y = home.y - home.height as i32;
            self.window
                .set_outer_position(PhysicalPosition::new(home.x, self.y));
            self.visible = true;
            self.window.set_visible(true);
        }
        self.hiding = false;
        self.slide = Some(Slide::toward(
            now,
            self.y,
            home.y,
            home.height,
            self.animation,
        ));
        self.window.set_focus();
        self.window.request_redraw();
        self.tick(now);
    }

    /// Slide the window back up and hide it. If it is partway in, it reverses from where it is.
    pub fn hide(&mut self, now: Instant) {
        let Some(home) = self.home.filter(|_| self.visible) else {
            return;
        };
        self.hiding = true;
        self.slide = Some(Slide::toward(
            now,
            self.y,
            home.y - home.height as i32,
            home.height,
            self.animation,
        ));
        self.tick(now);
    }

    /// Advance the running slide, if any. Call on every wake-up while [`is_animating`].
    ///
    /// [`is_animating`]: DropDown::is_animating
    pub fn tick(&mut self, now: Instant) {
        let (Some(slide), Some(home)) = (self.slide, self.home) else {
            return;
        };
        self.y = slide.y_at(now);
        self.window
            .set_outer_position(PhysicalPosition::new(home.x, self.y));
        if !slide.is_done(now) {
            return;
        }
        self.slide = None;
        if self.hiding {
            self.hiding = false;
            self.visible = false;
            self.window.set_visible(false);
            // Park at the resting spot so the next show finds the same monitor.
            self.y = home.y;
            self.window
                .set_outer_position(PhysicalPosition::new(home.x, home.y));
        }
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
