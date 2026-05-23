use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::process::Command;
use std::thread;

use crate::models::{
    AudioNode, EngineCommand, EngineEvent, LinkInfo, NodeType, PortDirection, PortInfo,
};

pub struct PipeWireEngine {
    _cmd_tx: Sender<EngineCommand>,
}

impl PipeWireEngine {
    pub fn new(event_tx: Sender<EngineEvent>) -> (Self, Sender<EngineCommand>) {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();

        // Spawn the monitoring thread
        let event_tx2 = event_tx.clone();
        thread::spawn(move || {
            if let Err(e) = run_monitor_thread(event_tx2) {
                eprintln!("PipeWire monitor thread error: {e}");
            }
        });

        // Spawn the command processing thread
        let event_tx3 = event_tx.clone();
        let cmd_rx2 = cmd_rx.clone();
        thread::spawn(move || {
            run_command_thread(cmd_rx2, event_tx3);
        });

        (PipeWireEngine { _cmd_tx: cmd_tx.clone() }, cmd_tx)
    }
}

fn run_monitor_thread(
    event_tx: Sender<EngineEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Reset any nodes that were left muted by a previous crash
    reset_all_mutes();

    let event_tx_for_global = event_tx.clone();
    let event_tx_for_remove = event_tx.clone();
    pipewire::init();

    let main_loop = pipewire::main_loop::MainLoopRc::new(None)?;
    let context = pipewire::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;

    let registry_weak = registry.downgrade();

    let _registry_listener = registry
        .add_listener_local()
        .global(move |obj| {
            let event_tx = event_tx_for_global.clone();
            if let Some(registry) = registry_weak.upgrade() {
                match obj.type_ {
                    pipewire::types::ObjectType::Node => {
                        if let Ok(_node) = registry.bind::<pipewire::node::Node, _>(obj) {
                            let props = obj.props.as_ref().map(|p| {
                                let mut map = HashMap::new();
                                for (k, v) in p.as_ref().iter() {
                                    map.insert(k.to_string(), v.to_string());
                                }
                                map
                            });

                            let id = obj.id;
                            let media_class = props
                                .as_ref()
                                .and_then(|m| m.get("media.class").cloned())
                                .unwrap_or_default();
                            let node_name = props
                                .as_ref()
                                .and_then(|m| m.get("node.name").cloned())
                                .unwrap_or_else(|| format!("node-{id}"));
                            let description = props
                                .as_ref()
                                .and_then(|m| {
                                    m.get("node.description")
                                        .or_else(|| m.get("node.nick"))
                                        .or_else(|| m.get("media.name"))
                                        .cloned()
                                })
                                .unwrap_or_else(|| node_name.clone());

                            let node_type = classify_node(&media_class, &node_name);

                            let event = EngineEvent::NodeAdded(AudioNode {
                                id,
                                name: node_name,
                                description,
                                node_type,
                                volume: 1.0,
                                muted: false,
                            });
                            let _ = event_tx.send(event);
                        }
                    }
                    pipewire::types::ObjectType::Port => {
                        let props = obj.props.as_ref().map(|p| {
                            let mut map = HashMap::new();
                            for (k, v) in p.as_ref().iter() {
                                map.insert(k.to_string(), v.to_string());
                            }
                            map
                        });

                        let id = obj.id;
                        let node_id = props
                            .as_ref()
                            .and_then(|m| m.get("node.id").and_then(|s| s.parse().ok()))
                            .unwrap_or(0);
                        let port_name = props
                            .as_ref()
                            .and_then(|m| m.get("port.name").cloned())
                            .unwrap_or_else(|| format!("port-{id}"));
                        let direction = props
                            .as_ref()
                            .and_then(|m| m.get("port.direction"))
                            .map(|d| {
                                if d == "out" {
                                    PortDirection::Output
                                } else {
                                    PortDirection::Input
                                }
                            })
                            .unwrap_or(PortDirection::Input);

                        let event = EngineEvent::PortAdded(PortInfo {
                            id,
                            node_id,
                            name: port_name,
                            direction,
                        });
                        let _ = event_tx.send(event);
                    }
                    pipewire::types::ObjectType::Link => {
                        let props = obj.props.as_ref().map(|p| {
                            let mut map = HashMap::new();
                            for (k, v) in p.as_ref().iter() {
                                map.insert(k.to_string(), v.to_string());
                            }
                            map
                        });

                        let id = obj.id;
                        let output_node = props
                            .as_ref()
                            .and_then(|m| m.get("link.output.node").and_then(|s| s.parse().ok()))
                            .unwrap_or(0);
                        let input_node = props
                            .as_ref()
                            .and_then(|m| m.get("link.input.node").and_then(|s| s.parse().ok()))
                            .unwrap_or(0);
                        let output_port = props
                            .as_ref()
                            .and_then(|m| m.get("link.output.port").and_then(|s| s.parse().ok()))
                            .unwrap_or(0);
                        let input_port = props
                            .as_ref()
                            .and_then(|m| m.get("link.input.port").and_then(|s| s.parse().ok()))
                            .unwrap_or(0);

                        let event = EngineEvent::LinkAdded(LinkInfo {
                            id,
                            output_node,
                            input_node,
                            output_port,
                            input_port,
                        });
                        let _ = event_tx.send(event);
                    }
                    _ => {}
                }
            }
        })
        .global_remove(move |id| {
            let _ = event_tx_for_remove.send(EngineEvent::NodeRemoved(id));
            let _ = event_tx_for_remove.send(EngineEvent::PortRemoved(id));
            let _ = event_tx_for_remove.send(EngineEvent::LinkRemoved(id));
        })
        .register();

    // Run forever
    main_loop.run();

    unsafe { pipewire::deinit(); }
    Ok(())
}

fn run_command_thread(cmd_rx: Receiver<EngineCommand>, event_tx: Sender<EngineEvent>) {
    while let Ok(cmd) = cmd_rx.recv() {
        match cmd {
            EngineCommand::SetVolume { node_id, volume } => {
                let _ = Command::new("pw-cli")
                    .args([
                        "set-param",
                        &format!("{node_id}"),
                        "Props",
                        &format!("{{ channelVolumes: [ {volume:.3} ] }}"),
                    ])
                    .output();
                let _ = event_tx.send(EngineEvent::VolumeChanged { node_id, volume });
            }
            EngineCommand::SetMute { node_id, muted } => {
                let mute_str = if muted { "true" } else { "false" };
                let _ = Command::new("pw-cli")
                    .args([
                        "set-param",
                        &format!("{node_id}"),
                        "Props",
                        &format!("{{ mute: {mute_str} }}"),
                    ])
                    .output();
                let _ = event_tx.send(EngineEvent::MuteChanged { node_id, muted });
            }
            EngineCommand::CreateLink { from_node, to_node } => {
                create_link_pw_cli(from_node, to_node);
            }
            EngineCommand::RemoveLinks { link_ids } => {
                for link_id in link_ids {
                    let _ = Command::new("pw-cli")
                        .args(["destroy", &format!("{link_id}")])
                        .output();
                }
            }
            EngineCommand::LoadNullSink { name } => {
                let _ = Command::new("pw-cli")
                    .args([
                        "load-module",
                        "module-null-sink",
                        &format!("sink_name={name}"),
                        &format!("sink_properties=device.description={name}"),
                    ])
                    .output();
            }
            EngineCommand::UnloadModule { module_id } => {
                let _ = Command::new("pw-cli")
                    .args(["destroy", &format!("{module_id}")])
                    .output();
            }
        }
    }
}

fn create_link_pw_cli(from_node: u32, to_node: u32) {
    let output = match Command::new("pw-cli").args(["ls", "Port"]).output() {
        Ok(o) => o,
        Err(_) => return,
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let mut ports: Vec<(u32, u32, String, String)> = vec![]; // id, node_id, direction, name
    let mut current_id = None;

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("id ") {
            if let Some(id_str) = trimmed.split_whitespace().nth(1) {
                current_id = id_str.trim_end_matches(',').parse().ok();
            }
        } else if let Some(id) = current_id {
            if trimmed.starts_with("node.id = ") {
                if let Some(val) = trimmed.split("= \"").nth(1) {
                    if let Ok(node_id) = val.trim_end_matches('"').parse::<u32>() {
                        if node_id == from_node || node_id == to_node {
                            ports.push((id, node_id, String::new(), String::new()));
                        }
                    }
                }
            }
        }
    }

    // Second pass: get directions
    let mut current_id2: Option<u32> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("id ") {
            if let Some(id_str) = trimmed.split_whitespace().nth(1) {
                current_id2 = id_str.trim_end_matches(',').parse().ok();
            }
        } else if let Some(id) = current_id2 {
            if trimmed.starts_with("port.direction = ") {
                if let Some(dir) = trimmed.split("= \"").nth(1) {
                    let dir = dir.trim_end_matches('"');
                    if let Some(p) = ports.iter_mut().find(|p| p.0 == id) {
                        p.2 = dir.to_string();
                    }
                }
            }
            if trimmed.starts_with("port.name = ") {
                if let Some(name) = trimmed.split("= \"").nth(1) {
                    let name = name.trim_end_matches('"');
                    if let Some(p) = ports.iter_mut().find(|p| p.0 == id) {
                        p.3 = name.to_string();
                    }
                }
            }
        }
    }

    let out_ports: Vec<_> = ports
        .iter()
        .filter(|p| p.1 == from_node && p.2 == "out")
        .collect();
    let in_ports: Vec<_> = ports
        .iter()
        .filter(|p| p.1 == to_node && p.2 == "in")
        .collect();

    // Pair up ports by matching channel names (e.g., FL with FL, FR with FR)
    for out_port in &out_ports {
        let matching_in = in_ports.iter().find(|in_port| {
            out_port.3 == in_port.3
                || (out_port.3.contains("FL") && in_port.3.contains("FL"))
                || (out_port.3.contains("FR") && in_port.3.contains("FR"))
                || (out_port.3.contains("MONO") && in_port.3.contains("MONO"))
        });

        if let Some(in_port) = matching_in {
            let _ = Command::new("pw-cli")
                .args([
                    "create-link",
                    &format!("{}", from_node),
                    &format!("{}", to_node),
                    &format!("{}", out_port.0),
                    &format!("{}", in_port.0),
                ])
                .output();
        } else if in_ports.len() == 1 {
            // If only one input port, connect everything to it (mixdown)
            let _ = Command::new("pw-cli")
                .args([
                    "create-link",
                    &format!("{}", from_node),
                    &format!("{}", to_node),
                    &format!("{}", out_port.0),
                    &format!("{}", in_ports[0].0),
                ])
                .output();
        }
    }
}

fn classify_node(media_class: &str, node_name: &str) -> NodeType {
    match media_class {
        "Audio/Sink" => NodeType::OutputDevice,
        "Audio/Source" => NodeType::InputDevice,
        "Stream/Output/Audio" => NodeType::App,
        "Stream/Input/Audio" => NodeType::AppInput,
        _ => {
            if node_name.contains("null-sink") || node_name.contains("MusicMixer") {
                NodeType::VirtualBus
            } else {
                NodeType::Other
            }
        }
    }
}

/// On startup, find all stream/app nodes and unmute them + restore volume to 1.0.
/// This recovers from a previous crash that left nodes muted.
fn reset_all_mutes() {
    let output = match Command::new("pw-cli").args(["ls", "Node"]).output() {
        Ok(o) => o,
        Err(_) => return,
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut current_id: Option<u32> = None;
    let mut current_class = String::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("id ") {
            if let Some(id_str) = trimmed.split_whitespace().nth(1) {
                current_id = id_str.trim_end_matches(',').parse().ok();
                current_class.clear();
            }
        } else if trimmed.starts_with("media.class = ") {
            if let Some(val) = trimmed.split("= \"").nth(1) {
                current_class = val.trim_end_matches('"').to_string();
            }
        } else if trimmed.starts_with("node.name = ") {
            // Once we have both id and class, reset if it's an app stream
            if let Some(id) = current_id {
                let is_stream = matches!(
                    current_class.as_str(),
                    "Stream/Output/Audio" | "Stream/Input/Audio"
                );
                if is_stream {
                    let _ = Command::new("pw-cli")
                        .args([
                            "set-param", &format!("{id}"), "Props",
                            "{ mute: false }",
                        ])
                        .output();
                    let _ = Command::new("pw-cli")
                        .args([
                            "set-param", &format!("{id}"), "Props",
                            "{ channelVolumes: [ 1.000 ] }",
                        ])
                        .output();
                }
            }
        }
    }
}
