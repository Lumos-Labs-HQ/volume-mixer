use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    App,          // Stream/Output/Audio — app playing audio
    AppInput,     // Stream/Input/Audio — app recording audio
    OutputDevice, // Audio/Sink — speakers, headphones
    InputDevice,  // Audio/Source — microphone
    VirtualBus,   // Null sink created by mixer
    Other,
}

#[derive(Debug, Clone)]
pub struct AudioNode {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub node_type: NodeType,
    pub volume: f32,   // 0.0 - 1.0
    pub muted: bool,
    pub pulse_id: Option<u32>, // object.serial — matches pactl sink-input/source-output index
}

#[derive(Debug, Clone)]
pub struct PortInfo {
    pub id: u32,
    pub node_id: u32,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub direction: PortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone)]
pub struct LinkInfo {
    pub id: u32,
    pub output_node: u32,
    pub input_node: u32,
    #[allow(dead_code)]
    pub output_port: u32,
    #[allow(dead_code)]
    pub input_port: u32,
}

#[derive(Debug, Clone, Default)]
pub struct MixerState {
    pub nodes: HashMap<u32, AudioNode>,
    pub ports: HashMap<u32, PortInfo>,
    pub links: HashMap<u32, LinkInfo>,
    pub node_ports: HashMap<u32, Vec<u32>>, // node_id -> port_ids
}

/// A unified application that may have both playback and capture streams.
#[derive(Debug, Clone)]
pub struct AppGroup {
    pub name: String,          // display name (e.g. "Discord")
    pub node_name: String,     // raw pw node.name for pw-link (e.g. "WEBRTC VoiceEngine")
    pub playback_id: Option<u32>,
    pub capture_id: Option<u32>,
    pub volume: f32,
    pub muted: bool,
    pub icon: AppIcon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppIcon {
    Music,
    Mic,
    Speaker,
}

impl MixerState {
    pub fn playback_apps(&self) -> Vec<&AudioNode> {
        self.nodes
            .values()
            .filter(|n| matches!(n.node_type, NodeType::App))
            .collect()
    }

    pub fn output_devices(&self) -> Vec<&AudioNode> {
        self.nodes
            .values()
            .filter(|n| matches!(n.node_type, NodeType::OutputDevice | NodeType::VirtualBus))
            .collect()
    }

    pub fn input_devices(&self) -> Vec<&AudioNode> {
        self.nodes
            .values()
            .filter(|n| matches!(n.node_type, NodeType::InputDevice))
            .collect()
    }

    pub fn app_inputs(&self) -> Vec<&AudioNode> {
        self.nodes
            .values()
            .filter(|n| matches!(n.node_type, NodeType::AppInput))
            .collect()
    }

    /// Group App (playback) and AppInput (capture) nodes by their display name.
    /// Returns deduplicated app groups sorted by name.
    pub fn app_groups(&self) -> Vec<AppGroup> {
        let mut groups: HashMap<String, AppGroup> = HashMap::new();

        // First pass: add all playback apps
        for node in self.playback_apps() {
            let name = node.description.clone();
            let group = groups.entry(name.clone()).or_insert_with(|| AppGroup {
                name: name.clone(),
                node_name: node.name.clone(),
                playback_id: None,
                capture_id: None,
                volume: node.volume,
                muted: node.muted,
                icon: AppIcon::Music,
            });
            group.playback_id = Some(node.id);
            group.node_name = node.name.clone();
            group.volume = node.volume;
            group.muted = node.muted;
        }

        // Second pass: add/merge capture apps
        for node in self.app_inputs() {
            let name = node.description.clone();
            let group = groups.entry(name.clone()).or_insert_with(|| AppGroup {
                name: name.clone(),
                node_name: node.name.clone(),
                playback_id: None,
                capture_id: None,
                volume: node.volume,
                muted: node.muted,
                icon: AppIcon::Mic,
            });
            group.capture_id = Some(node.id);
            if group.playback_id.is_none() {
                group.node_name = node.name.clone();
                group.volume = node.volume;
                group.muted = node.muted;
            }
        }

        // Determine icon: speaker if both playback+capture, mic if only capture, music default
        for group in groups.values_mut() {
            group.icon = if group.playback_id.is_some() && group.capture_id.is_some() {
                AppIcon::Speaker
            } else if group.capture_id.is_some() {
                AppIcon::Mic
            } else {
                AppIcon::Music
            };
        }

        // Deduplicate: if multiple playback nodes share a name, keep the one with highest pulse_id
        let mut deduped: HashMap<String, AppGroup> = HashMap::new();
        for (name, group) in groups {
            let existing = deduped.get(&name);
            let should_replace = match existing {
                None => true,
                Some(existing_group) => {
                    let existing_pid = existing_group.playback_id
                        .and_then(|id| self.nodes.get(&id))
                        .and_then(|n| n.pulse_id)
                        .unwrap_or(0);
                    let new_pid = group.playback_id
                        .and_then(|id| self.nodes.get(&id))
                        .and_then(|n| n.pulse_id)
                        .unwrap_or(0);
                    new_pid > existing_pid
                }
            };
            if should_replace {
                deduped.insert(name, group);
            }
        }

        let mut result: Vec<AppGroup> = deduped.into_values().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    /// Find the input device node ID linked to a given capture (AppInput) node.
    /// In PipeWire, capture links go: InputDevice (output) -> AppInput (input).
    /// So we look for links where input_node == capture_id and return output_node.
    #[allow(dead_code)]
    pub fn capture_input_device(&self, capture_id: u32) -> Option<u32> {
        self.links.values()
            .find(|l| l.input_node == capture_id)
            .map(|l| l.output_node)
    }

    /// Find all output device node IDs linked from a given playback (App) node.
    /// Deduplicated — PipeWire creates one link per port (FL/FR), so the same
    /// output device can appear multiple times; we return unique node IDs only.
    pub fn playback_output_devices(&self, playback_id: u32) -> Vec<u32> {
        let mut seen = std::collections::HashSet::new();
        self.links.values()
            .filter(|l| l.output_node == playback_id)
            .map(|l| l.input_node)
            .filter(|id| seen.insert(*id))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    NodeAdded(AudioNode),
    #[allow(dead_code)]
    NodeChanged(AudioNode),
    NodeRemoved(u32),
    PortAdded(PortInfo),
    PortRemoved(u32),
    LinkAdded(LinkInfo),
    LinkRemoved(u32),
    VolumeChanged { node_id: u32, volume: f32 },
    MuteChanged { node_id: u32, muted: bool },
}

#[derive(Debug, Clone)]
pub enum EngineCommand {
    /// volume 0.0-1.0; pulse_id = object.serial for streams, None for devices (use node_name)
    SetVolume { node_id: u32, volume: f32, pulse_id: Option<u32>, node_name: String, is_stream: bool },
    SetMute   { node_id: u32, muted: bool, pulse_id: Option<u32>, node_name: String, is_stream: bool },
    CreateLink { from_name: String, to_name: String },
    RemoveLink { from_name: String, to_name: String },
    /// Re-apply a list of (app_node_name, sink_node_name) links after a device reconnects
    RestoreLinks { pairs: Vec<(String, String)> },
    #[allow(dead_code)]
    LoadNullSink { name: String },
    #[allow(dead_code)]
    UnloadModule { module_id: u32 },
}
