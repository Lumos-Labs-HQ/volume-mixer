use gpui::{
    App, Application, AssetSource, Bounds, SharedString, TitlebarOptions, WindowDecorations,
    WindowOptions, prelude::*, px, size,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod audio;
mod models;
mod state;
mod ui;

use audio::PipeWireEngine;
use models::MixerState;
use state::MixerGlobal;
use ui::MixerWindow;

struct Assets {
    base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        std::fs::read(self.base.join(path))
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(|err| err.into())
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        std::fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|err| err.into())
    }
}

fn main() {
    Application::new()
        .with_assets(Assets {
            base: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
        })
        .run(|cx: &mut App| {
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
                        window_min_size: Some(size(px(720.), px(480.))),
                        titlebar: Some(TitlebarOptions {
                            title: Some("PipeWire Mixer".into()),
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
