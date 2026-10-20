//! Menubar app: a tray icon with About and Quit.

use quake_terminal::{about_metadata, build_icon};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
#[cfg(target_os = "macos")]
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::{Menu, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

fn main() {
    let mut event_loop = EventLoopBuilder::new().build();

    // Menubar-only app: no Dock icon.
    #[cfg(target_os = "macos")]
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    let menu = Menu::new();
    menu.append_items(&[
        &PredefinedMenuItem::about(None, Some(about_metadata())),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::quit(None),
    ])
    .expect("failed to build menu");

    // The tray icon must be created once the event loop has started (required on macOS),
    // and must be kept alive for as long as the loop runs.
    let mut tray: Option<TrayIcon> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::NewEvents(StartCause::Init) = event {
            let builder = TrayIconBuilder::new()
                .with_menu(Box::new(menu.clone()))
                .with_tooltip(env!("CARGO_PKG_NAME"));
            // Template icons are recoloured by macOS for light/dark menubars.
            #[cfg(target_os = "macos")]
            let builder = builder.with_icon_templated(build_icon());
            #[cfg(not(target_os = "macos"))]
            let builder = builder.with_icon(build_icon());
            tray = Some(builder.build().expect("failed to create tray icon"));
        }
        let _ = &tray;
    });
}
