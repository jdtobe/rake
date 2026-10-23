//! The drop-down window. This is OS glue around `tao` and the terminal webview, so it is verified
//! by hand rather than unit tested; placement maths lives in [`crate::geometry`] and slide maths
//! in [`crate::animation`].

use std::rc::Rc;
use std::time::{Duration, Instant};

use tao::dpi::{PhysicalPosition, PhysicalSize};
use tao::event_loop::EventLoopWindowTarget;
use tao::window::{Window, WindowBuilder, WindowId};

use crate::animation::Slide;
use crate::config::Config;
use crate::geometry::{window_rect, Rect};
use crate::terminal_protocol::TerminalEvent;
use crate::terminal_view::TerminalView;
use crate::theme::TerminalTheme;
use crate::APP_NAME;

/// macOS keeps normal and floating windows below the menubar and silently clamps any frame that
/// would go above it, which flattens the whole slide into one position. A window at the menubar's
/// own level is not clamped, and the menubar is drawn in front of it, so the panel slides out
/// from under the menubar rather than over it.
#[cfg(target_os = "macos")]
mod macos {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSWindow};
    use tao::platform::macos::WindowExtMacOS;
    use tao::window::Window;

    /// `NSMainMenuWindowLevel`. One level lower (23) is clamped; one higher (25, status) draws
    /// over the menubar.
    const MENUBAR_LEVEL: isize = 24;

    fn ns_window(window: &Window) -> &NSWindow {
        // SAFETY: tao returns the window's live `NSWindow`, which outlives the borrow of `window`.
        unsafe { &*window.ns_window().cast::<NSWindow>() }
    }

    pub fn raise_to_menubar_level(window: &Window) {
        let window = ns_window(window);
        window.setLevel(MENUBAR_LEVEL);
        window.setHidesOnDeactivate(false);
    }

    pub fn focus(window: &Window) {
        let marker = MainThreadMarker::new().expect("GUI work runs on the main thread");
        NSApplication::sharedApplication(marker).activate();
        ns_window(window).makeKeyAndOrderFront(None);
        window.set_focus();
    }

    /// Height in physical pixels of the strip at the top of the window's screen that is not
    /// usable (the menubar), or 0 if the screen is unknown.
    pub fn top_inset_px(window: &Window) -> u32 {
        let ns_window = ns_window(window);
        let Some(screen) = ns_window.screen() else {
            return 0;
        };
        let (frame, usable) = (screen.frame(), screen.visibleFrame());
        let points = (frame.origin.y + frame.size.height) - (usable.origin.y + usable.size.height);
        (points.max(0.0) * ns_window.backingScaleFactor()).round() as u32
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    use tao::window::Window;

    pub fn raise_to_menubar_level(_window: &Window) {}

    pub fn focus(window: &Window) {
        window.set_focus();
    }

    pub fn top_inset_px(_window: &Window) -> u32 {
        0
    }
}

pub struct DropDown {
    window: Rc<Window>,
    terminal: TerminalView,
    width_percent: u8,
    height_percent: u8,
    animation: Duration,
    /// Where the window rests while shown. Also where it is parked while hidden, so the next
    /// `show` finds the same monitor.
    home: Option<Rect>,
    /// Top edge of the window when parked fully above its monitor, in physical pixels.
    hidden_y: i32,
    /// Current top edge of the window, in physical pixels.
    y: i32,
    visible: bool,
    /// The running slide is a slide out, so the window is hidden when it finishes.
    hiding: bool,
    slide: Option<Slide>,
}

impl DropDown {
    /// Create the window hidden.
    pub fn new<T: 'static, F>(
        target: &EventLoopWindowTarget<T>,
        config: &Config,
        theme: &TerminalTheme,
        ipc: F,
    ) -> Result<Self, String>
    where
        F: Fn(String) + 'static,
    {
        let window = WindowBuilder::new()
            .with_title(APP_NAME)
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top(true)
            .with_visible(false)
            .build(target)
            .map_err(|e| format!("could not create window: {e}"))?;
        macos::raise_to_menubar_level(&window);
        let window = Rc::new(window);
        let terminal = TerminalView::new(&window, theme, ipc)?;
        Ok(DropDown {
            window,
            terminal,
            width_percent: config.width_percent,
            height_percent: config.height_percent,
            animation: Duration::from_millis(config.animation_ms.into()),
            home: None,
            hidden_y: 0,
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
            macos::top_inset_px(&self.window),
            self.width_percent,
            self.height_percent,
        );
        self.home = Some(home);
        self.hidden_y = position.y - home.height as i32;
        self.window
            .set_inner_size(PhysicalSize::new(home.width, home.height));

        if !self.visible {
            self.y = self.hidden_y;
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
            self.travel(home),
            self.animation,
        ));
        macos::focus(&self.window);
        let _ = self.terminal.focus();
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
            self.hidden_y,
            self.travel(home),
            self.animation,
        ));
        self.tick(now);
    }

    /// Distance between the hidden and resting positions.
    fn travel(&self, home: Rect) -> u32 {
        home.y.abs_diff(self.hidden_y)
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

    pub fn resize_terminal_view(&self) -> Result<(), String> {
        self.terminal
            .set_bounds(self.window.inner_size(), self.window.scale_factor())
    }

    pub fn mark_terminal_ready(&mut self) {
        self.terminal.mark_ready();
    }

    pub fn queue_terminal_event(&mut self, event: TerminalEvent) {
        self.terminal.queue(event);
    }

    pub fn flush_terminal(&mut self) -> Result<(), String> {
        self.terminal.flush()
    }

    pub fn focus_terminal(&self) -> Result<(), String> {
        self.terminal.focus()
    }
}
