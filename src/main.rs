//! Menubar app: a tray icon with About and Quit, plus a global show/hide hotkey.

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use quake_terminal::config::{self, Config};
use quake_terminal::hotkey::{should_toggle, Visibility};
use quake_terminal::{about_metadata, build_icon};
use std::path::Path;
use tao::event::{Event, StartCause};
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

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                let builder = TrayIconBuilder::new()
                    .with_menu(Box::new(menu.clone()))
                    .with_tooltip(env!("CARGO_PKG_NAME"));
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
            }
            Event::UserEvent(AppEvent::Hotkey(e))
                if e.id() == hotkey.id() && should_toggle(e.state()) =>
            {
                visibility = visibility.toggled();
                eprintln!("{visibility:?}");
            }
            _ => {}
        }
        let _ = (&tray, &hotkey_manager);
    });
}
