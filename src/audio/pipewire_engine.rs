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

                            let pulse_id: Option<u32> = props
                                .as_ref()
                                .and_then(|m| m.get("object.serial").and_then(|s| s.parse().ok()));

                            // For stream nodes, get the real app binary name from pactl
                            let display_name = if matches!(node_type, NodeType::App | NodeType::AppInput) {
                                pulse_id
                                    .and_then(|pid| pactl_binary_name(pid))
                                    .unwrap_or_else(|| description.clone())
                            } else {
                                description.clone()
                            };

                            let event = EngineEvent::NodeAdded(AudioNode {
                                id,
                                name: node_name,
                                description: display_name,
                                node_type,
                                volume: 1.0,
                                muted: false,
                                pulse_id,
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
            EngineCommand::SetVolume { node_id, volume, pulse_id, node_name, is_stream } => {
                let pct = format!("{}%", (volume * 100.0).round() as u32);
                if is_stream {
                    if let Some(pid) = pulse_id {
                        let _ = Command::new("pactl")
                            .args(["set-sink-input-volume", &pid.to_string(), &pct])
                            .output();
                    }
                } else {
                    let _ = Command::new("pactl")
                        .args(["set-sink-volume", &node_name, &pct])
                        .output();
                    let _ = Command::new("pactl")
                        .args(["set-source-volume", &node_name, &pct])
                        .output();
                }
                let _ = event_tx.send(EngineEvent::VolumeChanged { node_id, volume });
            }
            EngineCommand::SetMute { node_id, muted, pulse_id, node_name, is_stream } => {
                let val = if muted { "1" } else { "0" };
                if is_stream {
                    if let Some(pid) = pulse_id {
                        let _ = Command::new("pactl")
                            .args(["set-sink-input-mute", &pid.to_string(), val])
                            .output();
                    }
                } else {
                    let _ = Command::new("pactl")
                        .args(["set-sink-mute", &node_name, val])
                        .output();
                    let _ = Command::new("pactl")
                        .args(["set-source-mute", &node_name, val])
                        .output();
                }
                let _ = event_tx.send(EngineEvent::MuteChanged { node_id, muted });
            }
            EngineCommand::CreateLink { from_name, to_name } => {
                link_nodes(&from_name, &to_name, false);
            }
            EngineCommand::RemoveLink { from_name, to_name } => {
                link_nodes(&from_name, &to_name, true);
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

/// On startup, unmute all sink-inputs and restore volume to 100%.
fn reset_all_mutes() {
    let output = match Command::new("pactl").args(["list", "short", "sink-inputs"]).output() {
        Ok(o) => o,
        Err(_) => return,
    };
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(id_str) = line.split_whitespace().next() {
            let _ = Command::new("pactl").args(["set-sink-input-mute", id_str, "0"]).output();
            let _ = Command::new("pactl").args(["set-sink-input-volume", id_str, "100%"]).output();
        }
    }
}

/// Look up the real process binary name for a stream node via pactl.
/// pactl's "Sink Input #N" index matches the node's object.serial (pulse_id).
/// Falls back to None if not found (e.g. source-outputs use different list).
fn pactl_binary_name(pulse_id: u32) -> Option<String> {
    // Try sink-inputs first (playback), then source-outputs (capture)
    for list_type in &["sink-inputs", "source-outputs"] {
        let out = Command::new("pactl").args(["list", list_type]).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let mut in_block = false;
        let mut binary: Option<String> = None;
        for line in text.lines() {
            let t = line.trim();
            // "Sink Input #856" or "Source Output #868"
            if t.contains(&format!("#{pulse_id}")) {
                in_block = true;
                binary = None;
            } else if in_block {
                if t.starts_with("application.process.binary = ") {
                    binary = t.split('"').nth(1).map(|s| s.to_string());
                } else if t.starts_with("application.name = ") && binary.is_none() {
                    binary = t.split('"').nth(1).map(|s| s.to_string());
                }
                // Next block starts
                if (t.starts_with("Sink Input #") || t.starts_with("Source Output #")) && !t.contains(&format!("#{pulse_id}")) {
                    break;
                }
            }
        }
        if binary.is_some() {
            return binary;
        }
    }
    None
}

/// Link or unlink all output ports of `from_node` to all input ports of `to_node`.
/// Uses `pw-link -o` / `pw-link -i` to enumerate real port names, then pairs them.
fn link_nodes(from_name: &str, to_name: &str, disconnect: bool) {
    let out_ports = pw_ports_for(from_name, "output");
    let in_ports  = pw_ports_for(to_name,   "input");
    if out_ports.is_empty() || in_ports.is_empty() { return; }

    // Pair by channel suffix (FL↔FL, FR↔FR, MONO↔anything), or round-robin
    for out_port in &out_ports {
        let suffix = channel_suffix(out_port);
        let target = in_ports.iter()
            .find(|p| channel_suffix(p) == suffix)
            .or_else(|| in_ports.first())
            .unwrap();
        let mut args = vec![];
        if disconnect { args.push("-d"); }
        args.push(out_port.as_str());
        args.push(target.as_str());
        let _ = Command::new("pw-link").args(&args).output();
    }
    // Also connect each in_port that didn't get matched (e.g. stereo sink ← mono source)
    if out_ports.len() == 1 {
        for in_port in in_ports.iter().skip(1) {
            let mut args = vec![];
            if disconnect { args.push("-d"); }
            args.push(out_ports[0].as_str());
            args.push(in_port.as_str());
            let _ = Command::new("pw-link").args(&args).output();
        }
    }
}

fn pw_ports_for(node_name: &str, direction: &str) -> Vec<String> {
    let flag = if direction == "output" { "-o" } else { "-i" };
    let out = match Command::new("pw-link").args([flag]).output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.starts_with(&format!("{node_name}:")))
        // exclude monitor ports
        .filter(|l| !l.contains(":monitor_"))
        .map(|l| l.to_string())
        .collect()
}

fn channel_suffix(port: &str) -> &str {
    port.rsplit('_').next().unwrap_or("")
}
