use std::path::PathBuf;

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItemBuilder};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

// Reference: tao.rs example from tray-icon crate https://github.com/tauri-apps/tray-icon/blob/d4078696edba67b0ab42cef67e6a421a0332c96f/examples/tao.rs

// tray_icon needs an event loop.
// The event loop has to be run in the same thread as the tray icon is created.
// See: https://docs.rs/tray-icon/0.14.3/tray_icon/#platform-specific-notes
pub(crate) fn run_tray_icon_loop(log_path: Option<PathBuf>) {
    let event_loop = EventLoopBuilder::new().build();

    let menu_item_open_log = MenuItemBuilder::new()
        .text("Open log")
        .enabled(log_path.is_some())
        .build();
    let menu_item_quit = MenuItemBuilder::new().text("Quit").enabled(true).build();
    let menu = Menu::with_items(&[&menu_item_open_log, &menu_item_quit]).unwrap();

    let mut tray_icon = None;

    let menu_recv = MenuEvent::receiver();
    let tray_recv = TrayIconEvent::receiver();

    event_loop.run(move |event, _, control_flow| {
        // To reduce CPU usage, event-driven processing is used instead of periodical polling.
        *control_flow = ControlFlow::Wait;

        // Only when the event loop has been started
        if let Event::NewEvents(StartCause::Init) = event {
            tray_icon = Some(
                TrayIconBuilder::new()
                    .with_icon(Icon::from_rgba(vec![0; 64 * 64 * 4], 64, 64).unwrap())
                    .with_tooltip("sbchargelimit")
                    .with_menu(Box::new(menu.clone()))
                    .build()
                    .unwrap(),
            );
        }

        if let Ok(event) = menu_recv.try_recv() {
            if event.id == menu_item_open_log.id() {
                if let Some(log_path) = &log_path {
                    if let Err(e) = open::that_detached(log_path) {
                        log::error!("Log open error: {:?}", e);
                    }
                }
            } else if event.id == menu_item_quit.id() {
                tray_icon.take();
                *control_flow = ControlFlow::Exit; // Tell tao to stop the event loop
            }
        }

        if let Ok(_event) = tray_recv.try_recv() {
            // println!("{:?}", event);
        }
    });
}
