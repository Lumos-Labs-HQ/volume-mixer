use gpui::{
    div, prelude::*, px, relative, rgb, App, Context, IntoElement, Render, SharedString, Window,
};

use crate::models::{EngineCommand, EngineEvent, NodeType};
use crate::state::MixerGlobal;

pub struct MixerWindow {
    virtual_bus_counter: u32,
}

impl MixerWindow {
    pub fn new() -> Self {
        MixerWindow {
            virtual_bus_counter: 0,
        }
    }

    fn on_add_bus(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.virtual_bus_counter += 1;
        let name = format!("MusicMixer-Bus-{}", self.virtual_bus_counter);
        if let Some(global) = cx.try_global::<MixerGlobal>() {
            let cmd = EngineCommand::LoadNullSink { name };
            let _ = global.cmd_tx.send(cmd);
        }
        cx.notify();
    }

    pub fn poll_events(&mut self, cx: &mut Context<Self>) {
        let events = if let Some(global) = cx.try_global::<MixerGlobal>() {
            let mut events = vec![];
            while let Ok(event) = global.event_rx.try_recv() {
                events.push(event);
            }
            events
        } else {
            vec![]
        };

        if !events.is_empty() {
            if let Some(global) = cx.try_global::<MixerGlobal>() {
                let mut state = global.state.lock().unwrap();
                for event in events {
                    match event {
                        EngineEvent::NodeAdded(node) => {
                            state.nodes.insert(node.id, node);
                        }
                        EngineEvent::NodeChanged(node) => {
                            state.nodes.insert(node.id, node);
                        }
                        EngineEvent::NodeRemoved(id) => {
                            state.nodes.remove(&id);
                            state.node_ports.remove(&id);
                        }
                        EngineEvent::PortAdded(port) => {
                            let node_id = port.node_id;
                            let port_id = port.id;
                            state.ports.insert(port.id, port);
                            state.node_ports.entry(node_id).or_default().push(port_id);
                        }
                        EngineEvent::PortRemoved(id) => {
                            if let Some(port) = state.ports.remove(&id) {
                                if let Some(ports) = state.node_ports.get_mut(&port.node_id) {
                                    ports.retain(|&p| p != id);
                                }
                            }
                        }
                        EngineEvent::LinkAdded(link) => {
                            state.links.insert(link.id, link);
                        }
                        EngineEvent::LinkRemoved(id) => {
                            state.links.remove(&id);
                        }
                        EngineEvent::VolumeChanged { node_id, volume } => {
                            if let Some(node) = state.nodes.get_mut(&node_id) {
                                node.volume = volume;
                            }
                        }
                        EngineEvent::MuteChanged { node_id, muted } => {
                            if let Some(node) = state.nodes.get_mut(&node_id) {
                                node.muted = muted;
                            }
                        }
                    }
                }
            }
            cx.notify();
        }
    }
}

impl Render for MixerWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll_events(cx);

        let (playback_apps, input_devices, output_devices, app_inputs) =
            if let Some(global) = cx.try_global::<MixerGlobal>() {
                let state = global.state.lock().unwrap();
                (
                    state.playback_apps().into_iter().map(|n| n.id).collect::<Vec<_>>(),
                    state.input_devices().into_iter().map(|n| n.id).collect::<Vec<_>>(),
                    state.output_devices().into_iter().map(|n| n.id).collect::<Vec<_>>(),
                    state.app_inputs().into_iter().map(|n| n.id).collect::<Vec<_>>(),
                )
            } else {
                (vec![], vec![], vec![], vec![])
            };

        let outputs_for_routing: Vec<_> = if let Some(global) = cx.try_global::<MixerGlobal>() {
            let state = global.state.lock().unwrap();
            state
                .output_devices()
                .into_iter()
                .map(|n| (n.id, n.name.clone()))
                .collect()
        } else {
            vec![]
        };

        _window.request_animation_frame();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x111827))
            .text_color(rgb(0xffffff))
            .child(
                div()
                    .h(px(40.))
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px(px(12.))
                    .bg(rgb(0x1f2937))
                    .border_b_1()
                    .border_color(rgb(0x374151))
                    .child(
                        div()
                            .text_base()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(0xffffff))
                            .child("MusicMixer"),
                    )
                    .child(
                        div()
                            .px(px(10.))
                            .py(px(4.))
                            .rounded_md()
                            .bg(rgb(0x3b82f6))
                            .text_color(rgb(0xffffff))
                            .text_xs()
                            .cursor_pointer()
                            .child("+ Bus")
                            .id("add-bus-btn")
                            .on_click(cx.listener(|this, _event, window, cx| {
                                this.on_add_bus(window, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .id("mixer-scroll")
                    .overflow_y_scroll()
                    .p(px(12.))
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .when(!playback_apps.is_empty(), |d| {
                        d.child(section_header("Playback Apps", rgb(0x10b981)))
                            .child(channel_row("playback", &playback_apps, &outputs_for_routing, cx))
                    })
                    .when(!app_inputs.is_empty(), |d| {
                        d.child(section_header("App Inputs", rgb(0x8b5cf6)))
                            .child(channel_row("appinput", &app_inputs, &outputs_for_routing, cx))
                    })
                    .when(!input_devices.is_empty(), |d| {
                        d.child(section_header("Input Devices", rgb(0x3b82f6)))
                            .child(channel_row("input", &input_devices, &outputs_for_routing, cx))
                    })
                    .when(!output_devices.is_empty(), |d| {
                        d.child(section_header("Output Devices", rgb(0xf59e0b)))
                            .child(channel_row("output", &output_devices, &outputs_for_routing, cx))
                    })
                    .when(
                        playback_apps.is_empty()
                            && app_inputs.is_empty()
                            && input_devices.is_empty()
                            && output_devices.is_empty(),
                        |d| {
                            d.flex().items_center().justify_center().child(
                                div()
                                    .text_color(rgb(0x6b7280))
                                    .text_sm()
                                    .child("Waiting for PipeWire audio devices..."),
                            )
                        },
                    ),
            )
    }
}

// top_bar is inlined in render() to use cx.listener() for the add-bus button

fn section_header(title: &str, color: gpui::Rgba) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(color)
        .child(SharedString::from(title.to_string()))
}

fn channel_row(
    section: &str,
    node_ids: &[u32],
    outputs: &[(u32, String)],
    cx: &App,
) -> impl IntoElement {
    let outputs = outputs.to_vec();
    let section_id = SharedString::from(format!("row-{}", section));
    div()
        .flex()
        .flex_row()
        .gap(px(8.))
        .id(section_id)
        .overflow_x_scroll()
        .children(node_ids.iter().map(move |&node_id| {
            let outputs = outputs.clone();
            render_channel_strip(node_id, outputs, cx)
        }))
}

fn get_node_info(cx: &App, node_id: u32) -> Option<(String, String, NodeType, f32, bool)> {
    cx.try_global::<MixerGlobal>().and_then(|global| {
        let state = global.state.lock().unwrap();
        state.nodes.get(&node_id).map(|n| {
            (
                n.name.clone(),
                n.description.clone(),
                n.node_type.clone(),
                n.volume,
                n.muted,
            )
        })
    })
}

fn render_channel_strip(
    node_id: u32,
    outputs: Vec<(u32, String)>,
    cx: &App,
) -> impl IntoElement {
    let (name, description, node_type, volume, muted) =
        get_node_info(cx, node_id).unwrap_or_else(|| {
            (
                format!("Node {}", node_id),
                format!("ID: {}", node_id),
                NodeType::Other,
                1.0,
                false,
            )
        });

    let header_color = match node_type {
        NodeType::InputDevice | NodeType::AppInput => rgb(0x3b82f6),
        NodeType::OutputDevice | NodeType::VirtualBus => rgb(0xf59e0b),
        _ => rgb(0x10b981),
    };

    let mute_bg = if muted { rgb(0xef4444) } else { rgb(0x4b5563) };
    let vol_percent = (volume * 100.0) as u32;
    let vol_width = px(100.0 * volume);

    let is_linked = |out_id: u32| -> bool {
        cx.try_global::<MixerGlobal>()
            .map(|global| {
                let state = global.state.lock().unwrap();
                state.is_linked(node_id, out_id)
            })
            .unwrap_or(false)
    };

    div()
        .w(px(130.))
        .h(px(300.))
        .flex()
        .flex_col()
        .border_1()
        .border_color(rgb(0x374151))
        .bg(rgb(0x1f2937))
        .rounded_md()
        .overflow_hidden()
        // Header
        .child(
            div()
                .h(px(28.))
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .px(px(4.))
                .bg(header_color)
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0xffffff))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(SharedString::from(truncate(&name, 16))),
                ),
        )
        // Description
        .child(
            div()
                .h(px(18.))
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .px(px(2.))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x9ca3af))
                        .line_height(relative(1.0))
                        .child(SharedString::from(truncate(&description, 20))),
                ),
        )
        // Volume section
        .child(
            div()
                .p(px(6.))
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .px(px(6.))
                                .py(px(2.))
                                .rounded_md()
                                .bg(mute_bg)
                                .text_color(rgb(0xffffff))
                                .text_xs()
                                .cursor_pointer()
                                .child(if muted { "Unmute" } else { "Mute" })
                                .id(SharedString::from(format!("mute-{}", node_id)))
                                .on_click(move |_event, _window, cx: &mut App| {
                                    let muted = cx
                                        .try_global::<MixerGlobal>()
                                        .and_then(|global| {
                                            let state = global.state.lock().unwrap();
                                            state.nodes.get(&node_id).map(|n| !n.muted)
                                        })
                                        .unwrap_or(false);
                                    if let Some(global) = cx.try_global::<MixerGlobal>() {
                                        let _ = global
                                            .cmd_tx
                                            .send(EngineCommand::SetMute { node_id, muted });
                                    }
                                }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xffffff))
                                .child(format!("{}%", vol_percent)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex_1()
                                .py(px(2.))
                                .rounded_md()
                                .bg(rgb(0x374151))
                                .text_color(rgb(0xffffff))
                                .text_xs()
                                .text_center()
                                .cursor_pointer()
                                .child("-")
                                .id(SharedString::from(format!("vol-down-{}", node_id)))
                                .on_click(move |_event, _window, cx: &mut App| {
                                    let new_vol = cx
                                        .try_global::<MixerGlobal>()
                                        .and_then(|global| {
                                            let state = global.state.lock().unwrap();
                                            state.nodes.get(&node_id).map(|n| (n.volume - 0.1).max(0.0))
                                        })
                                        .unwrap_or(0.0);
                                    if let Some(global) = cx.try_global::<MixerGlobal>() {
                                        let _ = global.cmd_tx.send(EngineCommand::SetVolume {
                                            node_id,
                                            volume: new_vol,
                                        });
                                    }
                                }),
                        )
                        .child(
                            div()
                                .flex_1()
                                .py(px(2.))
                                .rounded_md()
                                .bg(rgb(0x374151))
                                .text_color(rgb(0xffffff))
                                .text_xs()
                                .text_center()
                                .cursor_pointer()
                                .child("+")
                                .id(SharedString::from(format!("vol-up-{}", node_id)))
                                .on_click(move |_event, _window, cx: &mut App| {
                                    let new_vol = cx
                                        .try_global::<MixerGlobal>()
                                        .and_then(|global| {
                                            let state = global.state.lock().unwrap();
                                            state.nodes.get(&node_id).map(|n| (n.volume + 0.1).min(1.0))
                                        })
                                        .unwrap_or(1.0);
                                    if let Some(global) = cx.try_global::<MixerGlobal>() {
                                        let _ = global.cmd_tx.send(EngineCommand::SetVolume {
                                            node_id,
                                            volume: new_vol,
                                        });
                                    }
                                }),
                        ),
                ),
        )
        // Volume bar
        .child(
            div()
                .px(px(6.))
                .child(
                    div()
                        .h(px(4.))
                        .w_full()
                        .rounded_full()
                        .bg(rgb(0x374151))
                        .child(
                            div()
                                .h_full()
                                .rounded_full()
                                .bg(rgb(0x10b981))
                                .w(vol_width),
                        ),
                ),
        )
        // Routing section
        .child(
            div()
                .p(px(6.))
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x9ca3af))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("Route to:"),
                )
                .children(outputs.into_iter().map(move |(out_id, out_name)| {
                    let linked = is_linked(out_id);
                    let checkbox_bg = if linked {
                        rgb(0x10b981)
                    } else {
                        rgb(0x4b5563)
                    };

                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(4.))
                        .cursor_pointer()
                        .id(SharedString::from(format!("route-{}-{}", node_id, out_id)))
                        .on_click(move |_event, _window, cx: &mut App| {
                            let link_ids = cx
                                .try_global::<MixerGlobal>()
                                .map(|global| {
                                    let state = global.state.lock().unwrap();
                                    state.find_links(node_id, out_id)
                                })
                                .unwrap_or_default();

                            if let Some(global) = cx.try_global::<MixerGlobal>() {
                                if !link_ids.is_empty() {
                                    let _ = global
                                        .cmd_tx
                                        .send(EngineCommand::RemoveLinks { link_ids });
                                } else {
                                    let _ = global.cmd_tx.send(EngineCommand::CreateLink {
                                        from_node: node_id,
                                        to_node: out_id,
                                    });
                                }
                            }
                        })
                        .child(
                            div()
                                .size(px(12.))
                                .rounded_sm()
                                .bg(checkbox_bg)
                                .border_1()
                                .border_color(rgb(0x6b7280))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(if linked {
                                    div().text_color(rgb(0xffffff)).text_xs().child("✓")
                                } else {
                                    div().child("")
                                }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xd1d5db))
                                .child(SharedString::from(truncate(&out_name, 14))),
                        )
                })),
        )
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        s.chars().take(max_len).collect::<String>() + "..."
    } else {
        s.to_string()
    }
}
