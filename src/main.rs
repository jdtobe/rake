//! Menubar app: a tray icon with About and Quit, plus a global show/hide hotkey.

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use rake::clipboard;
use rake::config::{self, Config};
use rake::dropdown::DropDown;
use rake::hotkey::{should_toggle, Visibility};
use rake::pty::{default_login_shell, PtySession, TerminalSize};
use rake::terminal_profile;
use rake::terminal_protocol::{parse_request, TerminalEvent, TerminalRequest};
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
    TerminalRequest(String),
    Terminal(TerminalEvent),
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
    let theme = terminal_profile::from_default_terminal().unwrap_or_default();

    let proxy = event_loop.create_proxy();
    let hotkey_proxy = proxy.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |event| {
        let _ = hotkey_proxy.send_event(AppEvent::Hotkey(event));
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
    let mut visibility = Visibility::Shown;
    let mut dropdown: Option<DropDown> = None;
    let mut terminal: Option<PtySession> = None;
    let mut terminal_accepting_input = false;
    let mut show_on_launch = false;

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

                let ipc_proxy = proxy.clone();
                let created_dropdown = DropDown::new(target, &config, &theme, move |body| {
                    let _ = ipc_proxy.send_event(AppEvent::TerminalRequest(body));
                })
                .unwrap_or_else(|e| fail(&e));

                let pty_proxy = proxy.clone();
                terminal = Some(
                    PtySession::spawn(
                        default_login_shell(std::env::var_os("SHELL")),
                        TerminalSize::new(80, 24).expect("initial terminal size is positive"),
                        move |event| {
                            let _ = pty_proxy.send_event(AppEvent::Terminal(event));
                        },
                    )
                    .unwrap_or_else(|e| fail(&e)),
                );
                terminal_accepting_input = true;
                dropdown = Some(created_dropdown);
                show_on_launch = true;
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
            Event::UserEvent(AppEvent::TerminalRequest(body)) => match parse_request(&body) {
                Ok(TerminalRequest::Ready) => {
                    if let Some(dropdown) = &mut dropdown {
                        dropdown.mark_terminal_ready();
                        if let Err(error) = dropdown.focus_terminal() {
                            eprintln!("{error}");
                        }
                    }
                }
                Ok(TerminalRequest::Input(bytes)) if terminal_accepting_input => {
                    if let Some(session) = &mut terminal {
                        if let Err(error) = session.write(&bytes) {
                            terminal_accepting_input = false;
                            if let Some(dropdown) = &mut dropdown {
                                dropdown.queue_terminal_event(TerminalEvent::Failed(error));
                            }
                        }
                    }
                }
                Ok(TerminalRequest::Input(_)) => {}
                Ok(TerminalRequest::Resize { cols, rows }) => {
                    let size = TerminalSize::new(cols, rows)
                        .expect("terminal protocol rejects zero dimensions");
                    if let Some(session) = &mut terminal {
                        if let Err(error) = session.resize(size) {
                            terminal_accepting_input = false;
                            if let Some(dropdown) = &mut dropdown {
                                dropdown.queue_terminal_event(TerminalEvent::Failed(error));
                            }
                        }
                    }
                }
                Ok(TerminalRequest::Copy(text)) => {
                    if let Err(error) = clipboard::write(&text) {
                        eprintln!("{error}");
                    }
                }
                Ok(TerminalRequest::Paste) if terminal_accepting_input => match clipboard::read() {
                    Ok(Some(text)) if !text.is_empty() => {
                        if let Some(dropdown) = &mut dropdown {
                            dropdown.queue_terminal_event(TerminalEvent::Paste(text));
                        }
                    }
                    Ok(_) => {}
                    Err(error) => eprintln!("{error}"),
                },
                Ok(TerminalRequest::Paste) => {}
                Err(error) => eprintln!("{error}"),
            },
            Event::UserEvent(AppEvent::Terminal(event)) => {
                if matches!(
                    event,
                    TerminalEvent::Exited { .. } | TerminalEvent::Failed(_)
                ) {
                    terminal_accepting_input = false;
                }
                if let Some(dropdown) = &mut dropdown {
                    dropdown.queue_terminal_event(event);
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(_),
                ..
            } => {
                if let Some(dropdown) = dropdown.as_ref().filter(|d| d.window_id() == window_id) {
                    let _ = dropdown.resize_terminal_view();
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
            // Runs after every batch of events, including the timed wake-ups requested below.
            Event::MainEventsCleared => {
                if let Some(dropdown) = &mut dropdown {
                    if show_on_launch {
                        show_on_launch = false;
                        dropdown.show(Instant::now());
                    }
                    dropdown.tick(Instant::now());
                    if let Err(error) = dropdown.flush_terminal() {
                        eprintln!("{error}");
                    }
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
        let _ = (&tray, &hotkey_manager, &terminal);
    });
}
