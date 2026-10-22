//! Menubar app: a tray icon with About and Quit, plus a global show/hide hotkey.

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use rake::config::{self, Config};
use rake::dropdown::DropDown;
use rake::hotkey::{should_toggle, Visibility};
use rake::{about_metadata, build_icon, APP_NAME};
use std::path::Path;
use std::time::Instant;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
#[cfg(target_os = "macos")]
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::{Menu, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

/// Events other threads forward into the single `tao` event loop.
#[derive(Debug)]
enum AppEvent {
    Hotkey(GlobalHotKeyEvent),
}

/// Report an unrecoverable startup problem and exit.
fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

fn main() {
    let mut event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();

    // Menubar-only app: no Dock icon.
    #[cfg(target_os = "macos")]
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    let config = match std::env::var_os("HOME") {
        Some(home) => config::load(&config::config_path(Path::new(&home))),
        None => Ok(Config::default()),
    }
    .unwrap_or_else(|e| fail(&e));
    let hotkey = config.hotkey().unwrap_or_else(|e| fail(&e));

    let proxy = event_loop.create_proxy();
    GlobalHotKeyEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(AppEvent::Hotkey(event));
    }));

    let menu = Menu::new();
    menu.append_items(&[
        &PredefinedMenuItem::about(None, Some(about_metadata())),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::quit(None),
    ])
    .expect("failed to build menu");

    // The tray icon and hotkey manager must be created once the event loop has started
    // (required on macOS), and must be kept alive for as long as the loop runs.
    let mut tray: Option<TrayIcon> = None;
    let mut hotkey_manager: Option<GlobalHotKeyManager> = None;
    let mut visibility = Visibility::Hidden;
    let mut dropdown: Option<DropDown> = None;

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                let builder = TrayIconBuilder::new()
                    .with_menu(Box::new(menu.clone()))
                    .with_tooltip(APP_NAME);
                // Template icons are recoloured by macOS for light/dark menubars.
                #[cfg(target_os = "macos")]
                let builder = builder.with_icon_templated(build_icon());
                #[cfg(not(target_os = "macos"))]
                let builder = builder.with_icon(build_icon());
                tray = Some(builder.build().expect("failed to create tray icon"));

                let manager = GlobalHotKeyManager::new().expect("failed to start hotkey manager");
                if let Err(e) = manager.register(hotkey) {
                    fail(&format!(
                        "could not register hotkey {:?}: {e}",
                        config.hotkey
                    ));
                }
                hotkey_manager = Some(manager);

                dropdown = Some(DropDown::new(target, &config).unwrap_or_else(|e| fail(&e)));
            }
            Event::UserEvent(AppEvent::Hotkey(e))
                if e.id() == hotkey.id() && should_toggle(e.state()) =>
            {
                visibility = visibility.toggled();
                if let Some(dropdown) = &mut dropdown {
                    match visibility {
                        Visibility::Shown => dropdown.show(Instant::now()),
                        Visibility::Hidden => dropdown.hide(Instant::now()),
                    }
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(_),
                ..
            } => {
                if let Some(dropdown) = dropdown.as_ref().filter(|d| d.window_id() == window_id) {
                    dropdown.request_redraw();
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Focused(focused),
                ..
            } => {
                if let Some(dropdown) = dropdown.as_mut().filter(|d| d.window_id() == window_id) {
                    let next = visibility.after_focus_change(config.hide_on_focus_lost, focused);
                    if next != visibility {
                        visibility = next;
                        dropdown.hide(Instant::now());
                    }
                }
            }
            Event::RedrawRequested(window_id) => {
                if let Some(dropdown) = dropdown.as_mut().filter(|d| d.window_id() == window_id) {
                    dropdown.paint();
                }
            }
            // Runs after every batch of events, including the timed wake-ups requested below.
            Event::MainEventsCleared => {
                if let Some(dropdown) = &mut dropdown {
                    dropdown.tick(Instant::now());
                }
            }
            _ => {}
        }

        // Keep the loop spinning while sliding; otherwise sleep until something happens. A timed
        // wake-up (`WaitUntil`) is not used because macOS fires timers late for this
        // background-style app, in the worst case freezing the slide for hundreds of
        // milliseconds. Slide positions come from elapsed time, so the spin rate does not matter.
        if dropdown.as_ref().is_some_and(DropDown::is_animating) {
            *control_flow = ControlFlow::Poll;
        }
        let _ = (&tray, &hotkey_manager);
    });
}
