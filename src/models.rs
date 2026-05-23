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

    pub fn find_links(&self, from_node: u32, to_node: u32) -> Vec<u32> {
        self.links
            .values()
            .filter(|l| l.output_node == from_node && l.input_node == to_node)
            .map(|l| l.id)
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
    SetVolume { node_id: u32, volume: f32 },
    SetMute { node_id: u32, muted: bool },
    CreateLink { from_node: u32, to_node: u32 },
    RemoveLinks { link_ids: Vec<u32> },
    LoadNullSink { name: String },
    #[allow(dead_code)]
    UnloadModule { module_id: u32 },
}
