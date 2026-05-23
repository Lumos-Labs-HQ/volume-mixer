use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowDecorations, WindowOptions, prelude::*, px,
    size,
};
use std::sync::{Arc, Mutex};

mod audio;
mod models;
mod state;
mod ui;

use audio::PipeWireEngine;
use models::MixerState;
use state::MixerGlobal;
use ui::MixerWindow;

fn main() {
    Application::new().run(|cx: &mut App| {
        // Set up channels
        let (event_tx, event_rx) = crossbeam_channel::unbounded();

        // Create the PipeWire engine
        let (engine, cmd_tx) = PipeWireEngine::new(event_tx);

        // Create global state
        let mixer_global = MixerGlobal {
            state: Arc::new(Mutex::new(MixerState::default())),
            event_rx,
            cmd_tx,
            engine,
        };

        cx.set_global(mixer_global);

        // Open window with client-side decorations so we can draw our own
        // titlebar with close/minimize buttons and custom resize handles.
        let window = cx
            .open_window(
                WindowOptions {
                    window_decorations: Some(WindowDecorations::Client),
                    window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(1100.), px(720.)),
                        cx,
                    ))),
                    window_min_size: Some(size(px(600.), px(400.))),
                    titlebar: Some(TitlebarOptions {
                        title: Some("MusicMixer".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_window, cx| {
                    cx.new(|_cx| MixerWindow::new())
                },
            )
            .unwrap();

        window
            .update(cx, |_view, _window, cx| {
                cx.activate(true);
            })
            .unwrap();
    });
}
