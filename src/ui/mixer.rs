use gpui::{
    div, prelude::*, px, rgb, App, Context, IntoElement, Render, SharedString, Window,
};

use crate::models::{EngineCommand, EngineEvent};
use crate::state::MixerGlobal;

pub struct MixerWindow {
    virtual_bus_counter: u32,
    open_dropdown: Option<u32>,
}

impl MixerWindow {
    pub fn new() -> Self {
        MixerWindow { virtual_bus_counter: 0, open_dropdown: None }
    }

    fn on_add_bus(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.virtual_bus_counter += 1;
        let name = format!("MusicMixer-Bus-{}", self.virtual_bus_counter);
        if let Some(g) = cx.try_global::<MixerGlobal>() {
            let _ = g.cmd_tx.send(EngineCommand::LoadNullSink { name });
        }
        cx.notify();
    }

    fn poll_events(&mut self, cx: &mut Context<Self>) {
        let events: Vec<EngineEvent> = cx.try_global::<MixerGlobal>()
            .map(|g| { let mut v = vec![]; while let Ok(e) = g.event_rx.try_recv() { v.push(e); } v })
            .unwrap_or_default();
        if events.is_empty() { return; }
        if let Some(g) = cx.try_global::<MixerGlobal>() {
            let mut s = g.state.lock().unwrap();
            for ev in events {
                match ev {
                    EngineEvent::NodeAdded(n) | EngineEvent::NodeChanged(n) => { s.nodes.insert(n.id, n); }
                    EngineEvent::NodeRemoved(id) => { s.nodes.remove(&id); s.node_ports.remove(&id); }
                    EngineEvent::PortAdded(p) => {
                        let (nid, pid) = (p.node_id, p.id);
                        s.ports.insert(p.id, p);
                        s.node_ports.entry(nid).or_default().push(pid);
                    }
                    EngineEvent::PortRemoved(id) => {
                        if let Some(p) = s.ports.remove(&id) {
                            if let Some(v) = s.node_ports.get_mut(&p.node_id) { v.retain(|&x| x != id); }
                        }
                    }
                    EngineEvent::LinkAdded(l) => { s.links.insert(l.id, l); }
                    EngineEvent::LinkRemoved(id) => { s.links.remove(&id); }
                    EngineEvent::VolumeChanged { node_id, volume } => {
                        if let Some(n) = s.nodes.get_mut(&node_id) { n.volume = volume; }
                    }
                    EngineEvent::MuteChanged { node_id, muted } => {
                        if let Some(n) = s.nodes.get_mut(&node_id) { n.muted = muted; }
                    }
                }
            }
        }
        cx.notify();
    }
}

// ── palette ──────────────────────────────────────────────────────────────────
const BG: u32         = 0x0f1117;
const SURFACE: u32    = 0x1a1d27;
const CARD: u32       = 0x1e2130;
const BORDER: u32     = 0x2a2d3a;
const TEXT: u32       = 0xe2e8f0;
const DIM: u32        = 0x64748b;
const GREEN: u32      = 0x22c55e;
const BLUE: u32       = 0x3b82f6;
const AMBER: u32      = 0xf59e0b;
const PURPLE: u32     = 0xa855f7;
const RED: u32        = 0xef4444;
const BLUE_DIM: u32   = 0x1e3a5f;
const BLUE_TEXT: u32  = 0x93c5fd;

impl Render for MixerWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll_events(cx);
        _window.request_animation_frame();

        // snapshot everything we need before building the tree
        let (playback, app_inputs, inputs, outputs, out_list) =
            if let Some(g) = cx.try_global::<MixerGlobal>() {
                let s = g.state.lock().unwrap();
                let mut pb: Vec<u32> = s.playback_apps().iter().map(|n| n.id).collect();
                let mut ai: Vec<u32> = s.app_inputs().iter().map(|n| n.id).collect();
                let mut id: Vec<u32> = s.input_devices().iter().map(|n| n.id).collect();
                let mut od: Vec<u32> = s.output_devices().iter().map(|n| n.id).collect();
                pb.sort(); ai.sort(); id.sort(); od.sort();
                let ol: Vec<(u32, String)> = s.output_devices()
                    .iter().map(|n| (n.id, clip(&n.description, 22))).collect();
                (pb, ai, id, od, ol)
            } else { (vec![], vec![], vec![], vec![], vec![]) };

        // snapshot node data for all ids we'll render
        let all_ids: Vec<u32> = playback.iter().chain(app_inputs.iter())
            .chain(inputs.iter()).chain(outputs.iter()).copied().collect();
        let node_data: std::collections::HashMap<u32, (String, String, f32, bool)> =
            if let Some(g) = cx.try_global::<MixerGlobal>() {
                let s = g.state.lock().unwrap();
                all_ids.iter().filter_map(|&id| {
                    s.nodes.get(&id).map(|n| (id, (n.name.clone(), n.description.clone(), n.volume, n.muted)))
                }).collect()
            } else { Default::default() };

        // snapshot link state for routing
        let link_map: Vec<(u32, u32)> = if let Some(g) = cx.try_global::<MixerGlobal>() {
            let s = g.state.lock().unwrap();
            s.links.values().map(|l| (l.output_node, l.input_node)).collect()
        } else { vec![] };

        let open_dd = self.open_dropdown;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            // ── top bar (draggable) ───────────────────────────────────
            .child(
                div()
                    .h(px(48.))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(20.))
                    .bg(rgb(SURFACE))
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .id("titlebar")
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _cx| {
                        window.start_window_move();
                    })
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(15.))
                            .child("MusicMixer"),
                    )
                    .child(
                        div()
                            .px(px(14.)).py(px(6.))
                            .rounded_md()
                            .bg(rgb(BLUE))
                            .text_color(rgb(0xffffff))
                            .text_size(px(12.))
                            .cursor_pointer()
                            .child("+ Add Bus")
                            .id("add-bus-btn")
                            .on_click(cx.listener(|this, _, w, cx| this.on_add_bus(w, cx))),
                    ),
            )
            // ── body ─────────────────────────────────────────────────────
            .child(
                div()
                    .flex_1()
                    .id("body-scroll")
                    .overflow_y_scroll()
                    .p(px(20.))
                    .flex()
                    .flex_col()
                    .gap(px(28.))
                    .when(!playback.is_empty(), |d| {
                        d.child(render_section(
                            "[>] Playback Apps", GREEN, &playback, &out_list,
                            &node_data, &link_map, open_dd, cx,
                        ))
                    })
                    .when(!app_inputs.is_empty(), |d| {
                        d.child(render_section(
                            "[M] App Inputs", PURPLE, &app_inputs, &out_list,
                            &node_data, &link_map, open_dd, cx,
                        ))
                    })
                    .when(!inputs.is_empty(), |d| {
                        d.child(render_simple_section("[@] Input Devices", BLUE, &inputs, &node_data, cx))
                    })
                    .when(!outputs.is_empty(), |d| {
                        d.child(render_simple_section("[#] Output Devices", AMBER, &outputs, &node_data, cx))
                    })
                    .when(
                        playback.is_empty() && app_inputs.is_empty()
                            && inputs.is_empty() && outputs.is_empty(),
                        |d| d.flex().items_center().justify_center().child(
                            div().text_color(rgb(DIM)).child("Waiting for PipeWire…"),
                        ),
                    ),
            )
    }
}

// ── Section with routing ──────────────────────────────────────────────────────
fn render_section(
    title: &str,
    color: u32,
    ids: &[u32],
    out_list: &[(u32, String)],
    node_data: &std::collections::HashMap<u32, (String, String, f32, bool)>,
    link_map: &[(u32, u32)],
    open_dd: Option<u32>,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let mut row = div().flex().flex_row().flex_wrap().gap(px(12.));
    for &node_id in ids {
        let (name, desc, volume, muted) = node_data.get(&node_id)
            .cloned().unwrap_or_else(|| (format!("Node {node_id}"), String::new(), 1.0, false));
        let linked_outs: Vec<u32> = out_list.iter()
            .filter(|(oid, _)| link_map.iter().any(|(f, t)| *f == node_id && *t == *oid))
            .map(|(oid, _)| *oid).collect();
        let available: Vec<(u32, String)> = out_list.iter()
            .filter(|(oid, _)| !linked_outs.contains(oid)).cloned().collect();
        let card = routable_card(node_id, name, desc, volume, muted,
            &linked_outs, out_list, &available, open_dd == Some(node_id), cx);
        row = row.child(card);
    }
    div().flex().flex_col().gap(px(10.))
        .child(section_label(title, color))
        .child(row)
}

fn render_simple_section(
    title: &str,
    color: u32,
    ids: &[u32],
    node_data: &std::collections::HashMap<u32, (String, String, f32, bool)>,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let mut row = div().flex().flex_row().flex_wrap().gap(px(12.));
    for &node_id in ids {
        let (name, desc, volume, muted) = node_data.get(&node_id)
            .cloned().unwrap_or_else(|| (format!("Node {node_id}"), String::new(), 1.0, false));
        row = row.child(simple_card(node_id, name, desc, volume, muted, cx));
    }
    div().flex().flex_col().gap(px(10.))
        .child(section_label(title, color))
        .child(row)
}

fn section_label(title: &str, color: u32) -> impl IntoElement {
    div().flex().items_center().gap(px(8.))
        .child(div().w(px(3.)).h(px(16.)).rounded_full().bg(rgb(color)))
        .child(
            div()
                .text_color(rgb(color))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(12.))
                .child(SharedString::from(title.to_string())),
        )
}

// ── Routable card ─────────────────────────────────────────────────────────────
fn routable_card(
    node_id: u32,
    name: String,
    desc: String,
    volume: f32,
    muted: bool,
    linked_outs: &[u32],
    all_outputs: &[(u32, String)],
    available: &[(u32, String)],
    dd_open: bool,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let all_out = all_outputs.to_vec();
    let avail = available.to_vec();
    let linked = linked_outs.to_vec();

    // route pills
    let pills: Vec<_> = linked.iter().map(|&out_id| {
        let label = all_out.iter().find(|(id, _)| *id == out_id)
            .map(|(_, n)| n.clone()).unwrap_or_else(|| format!("{out_id}"));
        div()
            .flex().items_center().gap(px(4.))
            .px(px(7.)).py(px(2.))
            .rounded_full()
            .bg(rgb(BLUE_DIM))
            .border_1().border_color(rgb(BLUE))
            .child(div().text_color(rgb(BLUE_TEXT)).text_size(px(10.))
                .child(SharedString::from(label)))
            .child(
                div()
                    .text_color(rgb(DIM)).text_size(px(10.)).cursor_pointer()
                    .child("x")
                    .id(SharedString::from(format!("unroute-{node_id}-{out_id}")))
                    .on_click(move |_, _, cx: &mut App| {
                        let ids = cx.try_global::<MixerGlobal>()
                            .map(|g| g.state.lock().unwrap().find_links(node_id, out_id))
                            .unwrap_or_default();
                        if let Some(g) = cx.try_global::<MixerGlobal>() {
                            if !ids.is_empty() {
                                let _ = g.cmd_tx.send(EngineCommand::RemoveLinks { link_ids: ids });
                            }
                        }
                    }),
            )
    }).collect();

    // dropdown items
    let dd_items: Vec<_> = avail.iter().map(|(out_id, out_name)| {
        let out_id = *out_id;
        div()
            .px(px(12.)).py(px(7.))
            .cursor_pointer()
            .text_color(rgb(TEXT)).text_size(px(12.))
            .hover(|s| s.bg(rgb(0x2d3148)))
            .child(SharedString::from(out_name.clone()))
            .id(SharedString::from(format!("dd-item-{node_id}-{out_id}")))
            .on_click(move |_, _, cx: &mut App| {
                if let Some(g) = cx.try_global::<MixerGlobal>() {
                    let _ = g.cmd_tx.send(EngineCommand::CreateLink { from_node: node_id, to_node: out_id });
                }
            })
    }).collect();

    let has_avail = !avail.is_empty();

    div()
        .w(px(220.)).flex().flex_col()
        .bg(rgb(CARD)).border_1().border_color(rgb(BORDER)).rounded_lg().overflow_hidden()
        .child(card_header(&name, &desc))
        .child(vol_controls(node_id, volume, muted, cx))
        // routing
        .child(
            div().px(px(10.)).pb(px(10.)).flex().flex_col().gap(px(6.))
                .child(
                    div().text_color(rgb(DIM)).text_size(px(10.))
                        .font_weight(gpui::FontWeight::SEMIBOLD).child("OUTPUT ROUTING"),
                )
                .child(div().flex().flex_row().flex_wrap().gap(px(4.)).children(pills))
                .child(
                    div()
                        .flex().items_center().gap(px(4.))
                        .px(px(8.)).py(px(4.))
                        .rounded_md()
                        .bg(rgb(BLUE_DIM)).border_1().border_color(rgb(BLUE))
                        .cursor_pointer()
                        .text_color(rgb(BLUE_TEXT)).text_size(px(11.))
                        .child(if has_avail { "+ Add Output" } else { "All outputs added" })
                        .id(SharedString::from(format!("dd-btn-{node_id}")))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_dropdown = if this.open_dropdown == Some(node_id) { None } else { Some(node_id) };
                            cx.notify();
                        })),
                )
                .when(dd_open && !dd_items.is_empty(), |d| {
                    d.child(
                        div()
                            .bg(rgb(SURFACE)).border_1().border_color(rgb(BORDER)).rounded_md()
                            .children(dd_items),
                    )
                }),
        )
}

// ── Simple card ───────────────────────────────────────────────────────────────
fn simple_card(
    node_id: u32,
    name: String,
    desc: String,
    volume: f32,
    muted: bool,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    div()
        .w(px(200.)).flex().flex_col()
        .bg(rgb(CARD)).border_1().border_color(rgb(BORDER)).rounded_lg().overflow_hidden()
        .child(card_header(&name, &desc))
        .child(vol_controls(node_id, volume, muted, cx))
}

fn card_header(name: &str, desc: &str) -> impl IntoElement {
    // desc is the human-readable label (e.g. "Built-in Audio Analog Stereo")
    // name is the raw node name (e.g. "alsa_input.pci-...")
    let primary = if desc.is_empty() { name } else { desc };
    let secondary = if desc.is_empty() { "" } else { name };
    div().px(px(10.)).pt(px(10.)).pb(px(6.)).flex().flex_col().gap(px(2.))
        .child(
            div().text_color(rgb(TEXT)).font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(13.)).child(SharedString::from(clip(primary, 24))),
        )
        .when(!secondary.is_empty(), |d| d.child(
            div().text_color(rgb(DIM)).text_size(px(10.))
                .child(SharedString::from(clip(secondary, 28))),
        ))
}

fn vol_controls(
    node_id: u32,
    volume: f32,
    muted: bool,
    _cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let mute_bg = if muted { RED } else { 0x374151u32 };
    let vol_pct = (volume * 100.0).round() as u32;
    let bar_w = px(140.0 * volume);

    div().px(px(10.)).pb(px(10.)).flex().flex_col().gap(px(6.))
        // mute + percent
        .child(
            div().flex().items_center().justify_between()
                .child(
                    div()
                        .px(px(10.)).py(px(3.)).rounded_md()
                        .bg(rgb(mute_bg)).text_color(rgb(0xffffff)).text_size(px(11.))
                        .cursor_pointer()
                        .child(if muted { "Unmute" } else { "Mute" })
                        .id(SharedString::from(format!("mute-{node_id}")))
                        .on_click(move |_, _, cx: &mut App| {
                            let new_muted = cx.try_global::<MixerGlobal>()
                                .and_then(|g| g.state.lock().unwrap().nodes.get(&node_id).map(|n| !n.muted))
                                .unwrap_or(false);
                            if let Some(g) = cx.try_global::<MixerGlobal>() {
                                let _ = g.cmd_tx.send(EngineCommand::SetMute { node_id, muted: new_muted });
                            }
                        }),
                )
                .child(div().text_color(rgb(DIM)).text_size(px(11.)).child(format!("{vol_pct}%"))),
        )
        // − bar +
        .child(
            div().flex().items_center().gap(px(6.))
                .child(
                    div()
                        .w(px(22.)).py(px(3.)).rounded_md()
                        .bg(rgb(0x374151)).text_color(rgb(TEXT)).text_size(px(12.)).text_center()
                        .cursor_pointer().child("-")
                        .id(SharedString::from(format!("vd-{node_id}")))
                        .on_click(move |_, _, cx: &mut App| {
                            let v = cx.try_global::<MixerGlobal>()
                                .and_then(|g| g.state.lock().unwrap().nodes.get(&node_id).map(|n| (n.volume - 0.1).max(0.0)))
                                .unwrap_or(0.0);
                            if let Some(g) = cx.try_global::<MixerGlobal>() {
                                let _ = g.cmd_tx.send(EngineCommand::SetVolume { node_id, volume: v });
                            }
                        }),
                )
                .child(
                    div().flex_1().h(px(5.)).rounded_full().bg(rgb(0x374151))
                        .child(div().h_full().rounded_full().bg(rgb(GREEN)).w(bar_w)),
                )
                .child(
                    div()
                        .w(px(22.)).py(px(3.)).rounded_md()
                        .bg(rgb(0x374151)).text_color(rgb(TEXT)).text_size(px(12.)).text_center()
                        .cursor_pointer().child("+")
                        .id(SharedString::from(format!("vu-{node_id}")))
                        .on_click(move |_, _, cx: &mut App| {
                            let v = cx.try_global::<MixerGlobal>()
                                .and_then(|g| g.state.lock().unwrap().nodes.get(&node_id).map(|n| (n.volume + 0.1).min(1.0)))
                                .unwrap_or(1.0);
                            if let Some(g) = cx.try_global::<MixerGlobal>() {
                                let _ = g.cmd_tx.send(EngineCommand::SetVolume { node_id, volume: v });
                            }
                        }),
                ),
        )
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() > max { s.chars().take(max).collect::<String>() + "…" } else { s.to_string() }
}
