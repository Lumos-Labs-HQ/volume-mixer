use crossbeam_channel::{Receiver, Sender};
use gpui::Global;
use std::sync::{Arc, Mutex};

use crate::audio::PipeWireEngine;
use crate::models::{EngineCommand, EngineEvent, MixerState};

pub struct MixerGlobal {
    pub state: Arc<Mutex<MixerState>>,
    pub event_rx: Receiver<EngineEvent>,
    pub cmd_tx: Sender<EngineCommand>,
    #[allow(dead_code)]
    pub engine: PipeWireEngine,
}

impl Global for MixerGlobal {}
