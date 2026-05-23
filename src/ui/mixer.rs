use gpui::{
    div, prelude::*, px, rgb, App, Context, IntoElement, MouseButton, Render, ResizeEdge,
    SharedString, Window,
};

use crate::models::{EngineCommand, EngineEvent};
use crate::state::MixerGlobal;

pub struct MixerWindow {
    open_dropdown: Option<u32>,
}

impl MixerWindow {
    pub fn new() -> Self {
        MixerWindow { open_dropdown: None }
    }

    #[allow(dead_code)]
    fn on_add_bus(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

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
const BG: u32          = 0x0b0d12;
const SURFACE: u32     = 0x13161f;
const CARD: u32        = 0x181b25;
const CARD_HOVER: u32  = 0x1e2230;
const BORDER: u32      = 0x232736;
const BORDER_HOVER: u32 = 0x2d3144;
const TEXT: u32        = 0xe8eaf0;
const TEXT_MUTED: u32  = 0x7a8199;
const TEXT_DIM: u32    = 0x525766;
const GREEN: u32       = 0x34d399;
const BLUE: u32        = 0x60a5fa;
const AMBER: u32       = 0xfbbf24;
const PURPLE: u32      = 0xa78bfa;
const RED: u32         = 0xf87171;
const RED_BG: u32      = 0x3d1f1f;
const GREEN_BG: u32    = 0x1a2e26;
const SLIDER_TRACK: u32 = 0x2a2e3d;
const SLIDER_THUMB: u32 = 0xeef0f5;
const BTN_BG: u32      = 0x252a3a;
const BTN_BG_HOVER: u32 = 0x2d3144;
const ROUTE_BG: u32    = 0x1e2d4a;
const ROUTE_BORDER: u32 = 0x3b82f6;
const ROUTE_TEXT: u32  = 0x93c5fd;
const TITLEBAR_H: f32  = 40.;

impl Render for MixerWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll_events(cx);
        _window.request_animation_frame();

        // snapshot everything we need before building the tree
        let (playback, app_inputs, inputs, outputs, out_list, out_names) =
            if let Some(g) = cx.try_global::<MixerGlobal>() {
                let s = g.state.lock().unwrap();
                let mut pb: Vec<u32> = s.playback_apps().iter().map(|n| n.id).collect();
                let mut ai: Vec<u32> = s.app_inputs().iter().map(|n| n.id).collect();
                let mut id: Vec<u32> = s.input_devices().iter().map(|n| n.id).collect();
                let mut od: Vec<u32> = s.output_devices().iter().map(|n| n.id).collect();
                pb.sort(); ai.sort(); id.sort(); od.sort();
                let ol: Vec<(u32, String)> = s.output_devices()
                    .iter().map(|n| (n.id, clip(&n.description, 22))).collect();
                let on: std::collections::HashMap<u32, String> = s.output_devices()
                    .iter().map(|n| (n.id, n.name.clone())).collect();
                (pb, ai, id, od, ol, on)
            } else { (vec![], vec![], vec![], vec![], vec![], Default::default()) };

        let all_ids: Vec<u32> = playback.iter().chain(app_inputs.iter())
            .chain(inputs.iter()).chain(outputs.iter()).copied().collect();
        let node_data: std::collections::HashMap<u32, (String, String, f32, bool, Option<u32>, bool)> =
            if let Some(g) = cx.try_global::<MixerGlobal>() {
                let s = g.state.lock().unwrap();
                all_ids.iter().filter_map(|&id| {
                    s.nodes.get(&id).map(|n| {
                        let is_stream = matches!(n.node_type,
                            crate::models::NodeType::App | crate::models::NodeType::AppInput);
                        (id, (n.name.clone(), n.description.clone(), n.volume, n.muted, n.pulse_id, is_stream))
                    })
                }).collect()
            } else { Default::default() };

        let link_map: Vec<(u32, u32)> = if let Some(g) = cx.try_global::<MixerGlobal>() {
            let s = g.state.lock().unwrap();
            s.links.values().map(|l| (l.output_node, l.input_node)).collect()
        } else { vec![] };

        let open_dd = self.open_dropdown;

        div()
            .size_full()
            .relative()
            .flex().flex_col()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            // ── custom titlebar ─────────────────────────────────────────
            .child(titlebar(_window, cx))
            // ── body ────────────────────────────────────────────────────
            .child(
                div()
                    .flex_1()
                    .id("body-scroll")
                    .overflow_y_scroll()
                    .p(px(24.))
                    .flex()
                    .flex_col()
                    .gap(px(32.))
                    .when(!playback.is_empty(), |d| {
                        d.child(render_section(
                            "Playback Apps", "▶", GREEN, &playback, &out_list, &out_names,
                            &node_data, &link_map, open_dd, cx,
                        ))
                    })
                    .when(!app_inputs.is_empty(), |d| {
                        d.child(render_simple_section("App Inputs", "⏺", PURPLE, &app_inputs, &node_data, cx))
                    })
                    .when(!inputs.is_empty(), |d| {
                        d.child(render_simple_section("Input Devices", "🎙", BLUE, &inputs, &node_data, cx))
                    })
                    .when(!outputs.is_empty(), |d| {
                        d.child(render_simple_section("Output Devices", "🔈", AMBER, &outputs, &node_data, cx))
                    })
                    .when(
                        playback.is_empty() && app_inputs.is_empty()
                            && inputs.is_empty() && outputs.is_empty(),
                        |d| d.flex().items_center().justify_center().child(
                            div().flex().flex_col().items_center().gap(px(12.))
                                .child(div().text_size(px(28.)).text_color(rgb(TEXT_DIM)).child("♪"))
                                .child(div().text_color(rgb(TEXT_DIM)).text_size(px(13.)).child("Waiting for PipeWire…")),
                        ),
                    ),
            )
            // ── resize handles ──────────────────────────────────────────
            .child(
                div()
                    .absolute().right_0().top_0().bottom_0()
                    .w(px(4.))
                    .cursor_ew_resize()
                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                        window.start_window_resize(ResizeEdge::Right);
                    }),
            )
            .child(
                div()
                    .absolute().left_0().right_0().bottom_0()
                    .h(px(4.))
                    .cursor_ns_resize()
                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                        window.start_window_resize(ResizeEdge::Bottom);
                    }),
            )
            .child(
                div()
                    .absolute().right_0().bottom_0()
                    .w(px(10.)).h(px(10.))
                    .cursor_nwse_resize()
                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                        window.start_window_resize(ResizeEdge::BottomRight);
                    }),
            )
    }
}

fn titlebar(_window: &mut Window, _cx: &mut Context<MixerWindow>) -> impl IntoElement {
    div()
        .h(px(TITLEBAR_H))
        .px(px(16.))
        .flex().items_center().justify_between()
        .border_b_1().border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        // draggable region (left / center)
        .child(
            div()
                .flex().items_center().gap(px(10.))
                .flex_1()
                .h_full()
                .on_mouse_down(MouseButton::Left, |_, window, _| {
                    window.start_window_move();
                })
                .child(
                    div()
                        .w(px(24.)).h(px(24.))
                        .flex().items_center().justify_center()
                        .rounded_sm()
                        .bg(rgb(GREEN_BG))
                        .text_color(rgb(GREEN))
                        .text_size(px(12.))
                        .child("♪")
                )
                .child(
                    div()
                        .text_color(rgb(TEXT))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(13.))
                        .child("MusicMixer")
                ),
        )
        // window controls (right)
        .child(
            div().flex().items_center().gap(px(8.))
                .child(
                    div()
                        .w(px(28.)).h(px(28.))
                        .flex().items_center().justify_center()
                        .rounded_sm()
                        .text_color(rgb(TEXT_MUTED))
                        .text_size(px(14.))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(BTN_BG)).text_color(rgb(TEXT)))
                        .child("−")
                        .id("titlebar-minimize")
                        .on_click(|_, window, _| {
                            window.minimize_window();
                        }),
                )
                .child(
                    div()
                        .w(px(28.)).h(px(28.))
                        .flex().items_center().justify_center()
                        .rounded_sm()
                        .text_color(rgb(TEXT_MUTED))
                        .text_size(px(14.))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(RED_BG)).text_color(rgb(RED)))
                        .child("×")
                        .id("titlebar-close")
                        .on_click(|_, window, _| {
                            window.remove_window();
                        }),
                ),
        )
}

// ── Section with routing ──────────────────────────────────────────────────────
fn render_section(
    title: &str,
    icon: &str,
    color: u32,
    ids: &[u32],
    out_list: &[(u32, String)],
    out_names: &std::collections::HashMap<u32, String>,
    node_data: &std::collections::HashMap<u32, (String, String, f32, bool, Option<u32>, bool)>,
    link_map: &[(u32, u32)],
    open_dd: Option<u32>,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let mut row = div().flex().flex_row().flex_wrap().gap(px(14.));
    for &node_id in ids {
        let (name, desc, volume, muted, pulse_id, is_stream) = node_data.get(&node_id)
            .cloned().unwrap_or_else(|| (format!("Node {node_id}"), String::new(), 1.0, false, None, true));
        let linked_outs: Vec<u32> = out_list.iter()
            .filter(|(oid, _)| link_map.iter().any(|(f, t)| *f == node_id && *t == *oid))
            .map(|(oid, _)| *oid).collect();
        let available: Vec<(u32, String)> = out_list.iter()
            .filter(|(oid, _)| !linked_outs.contains(oid)).cloned().collect();
        let card = routable_card(node_id, name, desc, volume, muted, pulse_id, is_stream,
            &linked_outs, out_list, out_names, &available, open_dd == Some(node_id), cx);
        row = row.child(card);
    }
    div().flex().flex_col().gap(px(12.))
        .child(section_label(title, icon, color))
        .child(row)
}

fn render_simple_section(
    title: &str,
    icon: &str,
    color: u32,
    ids: &[u32],
    node_data: &std::collections::HashMap<u32, (String, String, f32, bool, Option<u32>, bool)>,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let mut row = div().flex().flex_row().flex_wrap().gap(px(14.));
    for &node_id in ids {
        let (name, desc, volume, muted, pulse_id, is_stream) = node_data.get(&node_id)
            .cloned().unwrap_or_else(|| (format!("Node {node_id}"), String::new(), 1.0, false, None, false));
        row = row.child(simple_card(node_id, name, desc, volume, muted, pulse_id, is_stream, cx));
    }
    div().flex().flex_col().gap(px(12.))
        .child(section_label(title, icon, color))
        .child(row)
}

fn section_label(title: &str, icon: &str, color: u32) -> impl IntoElement {
    div().flex().items_center().gap(px(10.))
        .child(
            div()
                .w(px(8.)).h(px(8.)).rounded_full()
                .bg(rgb(color)),
        )
        .child(
            div()
                .text_color(rgb(color))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(11.))
                .child(SharedString::from(title.to_string())),
        )
        .child(
            div()
                .text_color(rgb(TEXT_DIM))
                .text_size(px(11.))
                .child(SharedString::from(icon.to_string())),
        )
}

// ── Routable card ─────────────────────────────────────────────────────────────
fn routable_card(
    node_id: u32,
    name: String,
    desc: String,
    volume: f32,
    muted: bool,
    pulse_id: Option<u32>,
    is_stream: bool,
    linked_outs: &[u32],
    all_outputs: &[(u32, String)],
    out_names: &std::collections::HashMap<u32, String>,
    available: &[(u32, String)],
    dd_open: bool,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let all_out = all_outputs.to_vec();
    let avail = available.to_vec();
    let linked = linked_outs.to_vec();
    let out_name_map: std::collections::HashMap<u32, String> = out_names.clone();

    // route pills
    let pills: Vec<_> = linked.iter().map(|&out_id| {
        let label = all_out.iter().find(|(id, _)| *id == out_id)
            .map(|(_, n)| n.clone()).unwrap_or_else(|| format!("{out_id}"));
        let from_name = name.clone();
        let to_name = out_name_map.get(&out_id).cloned().unwrap_or_default();
        div()
            .flex().items_center().gap(px(5.))
            .px(px(8.)).py(px(3.))
            .rounded_sm()
            .bg(rgb(ROUTE_BG))
            .border_1().border_color(rgb(ROUTE_BORDER))
            .child(div().text_color(rgb(ROUTE_TEXT)).text_size(px(10.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(SharedString::from(label)))
            .child(
                div()
                    .w(px(14.)).h(px(14.))
                    .flex().items_center().justify_center()
                    .rounded_sm()
                    .hover(|s| s.bg(rgb(0x2d3a5f)))
                    .text_color(rgb(ROUTE_TEXT)).text_size(px(10.)).cursor_pointer()
                    .child("×")
                    .id(SharedString::from(format!("unroute-{node_id}-{out_id}")))
                    .on_click(move |_, _, cx: &mut App| {
                        if let Some(g) = cx.try_global::<MixerGlobal>() {
                            let _ = g.cmd_tx.send(EngineCommand::RemoveLink {
                                from_name: from_name.clone(),
                                to_name: to_name.clone(),
                            });
                        }
                    }),
            )
    }).collect();

    let dd_items: Vec<_> = avail.iter().map(|(out_id, out_name)| {
        let out_id = *out_id;
        let from_name = name.clone();
        let to_name = out_name_map.get(&out_id).cloned().unwrap_or_default();
        div()
            .px(px(12.)).py(px(7.))
            .cursor_pointer()
            .text_color(rgb(TEXT)).text_size(px(12.))
            .hover(|s| s.bg(rgb(0x252a3a)))
            .child(SharedString::from(out_name.clone()))
            .id(SharedString::from(format!("dd-item-{node_id}-{out_id}")))
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(g) = cx.try_global::<MixerGlobal>() {
                    let _ = g.cmd_tx.send(EngineCommand::CreateLink {
                        from_name: from_name.clone(),
                        to_name: to_name.clone(),
                    });
                }
                this.open_dropdown = None;
                cx.notify();
            }))
    }).collect();

    let has_avail = !avail.is_empty();

    div()
        .w(px(260.)).flex().flex_col()
        .bg(rgb(CARD)).border_1().border_color(rgb(BORDER)).rounded_md()
        .hover(|s| s.border_color(rgb(BORDER_HOVER)).bg(rgb(CARD_HOVER)))
        .child(card_header(&name, &desc))
        .child(vol_controls(node_id, name.clone(), volume, muted, pulse_id, is_stream, cx))
        // routing
        .child(
            div().px(px(14.)).pb(px(14.)).pt(px(4.)).flex().flex_col().gap(px(8.))
                .child(
                    div().text_color(rgb(TEXT_DIM)).text_size(px(9.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("OUTPUT ROUTING"),
                )
                .child(div().flex().flex_row().flex_wrap().gap(px(5.)).children(pills))
                .child(
                    div()
                        .flex().items_center().gap(px(5.))
                        .px(px(10.)).py(px(5.))
                        .rounded_sm()
                        .bg(rgb(ROUTE_BG)).border_1().border_color(rgb(ROUTE_BORDER))
                        .cursor_pointer()
                        .text_color(rgb(ROUTE_TEXT)).text_size(px(11.))
                        .font_weight(gpui::FontWeight::MEDIUM)
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
    pulse_id: Option<u32>,
    is_stream: bool,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    div()
        .w(px(260.)).flex().flex_col()
        .bg(rgb(CARD)).border_1().border_color(rgb(BORDER)).rounded_md()
        .hover(|s| s.border_color(rgb(BORDER_HOVER)).bg(rgb(CARD_HOVER)))
        .child(card_header(&name, &desc))
        .child(vol_controls(node_id, name, volume, muted, pulse_id, is_stream, cx))
}

fn card_header(name: &str, desc: &str) -> impl IntoElement {
    let primary = if desc.is_empty() { name } else { desc };
    let secondary = if desc.is_empty() { "" } else { name };
    div().px(px(14.)).pt(px(14.)).pb(px(8.)).flex().flex_col().gap(px(3.))
        .child(
            div().text_color(rgb(TEXT)).font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(13.)).line_height(px(18.))
                .child(SharedString::from(clip(primary, 26))),
        )
        .when(!secondary.is_empty(), |d| d.child(
            div().text_color(rgb(TEXT_MUTED)).text_size(px(10.)).line_height(px(14.))
                .child(SharedString::from(clip(secondary, 30))),
        ))
}

fn vol_controls(
    node_id: u32,
    node_name: String,
    volume: f32,
    muted: bool,
    #[allow(unused_variables)]
    pulse_id: Option<u32>,
    is_stream: bool,
    _cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let vol_pct = (volume * 100.0).round() as u32;
    let bar_w = px(120.0 * volume);
    let nn1 = node_name.clone();
    let nn2 = node_name.clone();
    let nn3 = node_name.clone();

    let bar_color = if volume > 0.8 { RED } else if volume > 0.5 { GREEN } else { GREEN };
    let mute_bg = if muted { RED_BG } else { BTN_BG };
    let mute_text = if muted { RED } else { TEXT_MUTED };
    let mute_label = if muted { "Unmute" } else { "Mute" };

    div().px(px(14.)).pb(px(14.)).flex().flex_col().gap(px(8.))
        // mute + percent row
        .child(
            div().flex().items_center().justify_between()
                .child(
                    div()
                        .px(px(10.)).py(px(4.))
                        .rounded_sm()
                        .bg(rgb(mute_bg))
                        .text_color(rgb(mute_text))
                        .text_size(px(10.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .cursor_pointer()
                        .hover(|s| if muted { s.bg(rgb(0x4a2020)) } else { s.bg(rgb(BTN_BG_HOVER)) })
                        .child(mute_label)
                        .id(SharedString::from(format!("mute-{node_id}")))
                        .on_click(move |_, _, cx: &mut App| {
                            let (new_muted, pid) = cx.try_global::<MixerGlobal>()
                                .and_then(|g| g.state.lock().unwrap().nodes.get(&node_id)
                                    .map(|n| (!n.muted, n.pulse_id)))
                                .unwrap_or((false, None));
                            if let Some(g) = cx.try_global::<MixerGlobal>() {
                                let _ = g.cmd_tx.send(EngineCommand::SetMute {
                                    node_id, muted: new_muted,
                                    pulse_id: pid, node_name: nn1.clone(), is_stream,
                                });
                            }
                        }),
                )
                .child(div().text_color(rgb(TEXT_MUTED)).text_size(px(11.)).font_weight(gpui::FontWeight::MEDIUM).child(format!("{vol_pct}%"))),
        )
        // slider row
        .child(
            div().flex().items_center().gap(px(8.))
                .child(
                    step_button("−", format!("vd-{node_id}"), move |_, _, cx: &mut App| {
                        let (v, pid) = cx.try_global::<MixerGlobal>()
                            .and_then(|g| g.state.lock().unwrap().nodes.get(&node_id)
                                .map(|n| ((n.volume - 0.05).max(0.0), n.pulse_id)))
                            .unwrap_or((0.0, None));
                        if let Some(g) = cx.try_global::<MixerGlobal>() {
                            let _ = g.cmd_tx.send(EngineCommand::SetVolume {
                                node_id, volume: v,
                                pulse_id: pid, node_name: nn2.clone(), is_stream,
                            });
                        }
                    }),
                )
                .child(
                    div().w(px(120.)).h(px(6.)).rounded_full().bg(rgb(SLIDER_TRACK))
                        .relative()
                        .child(
                            div().absolute().left_0().top_0().bottom_0()
                                .w(bar_w)
                                .rounded_full()
                                .bg(rgb(bar_color)),
                        )
                        .child(
                            div().absolute().top(px(-3.))
                                .left(bar_w)
                                .w(px(12.)).h(px(12.))
                                .rounded_full()
                                .bg(rgb(SLIDER_THUMB))
                                .border_1().border_color(rgb(SLIDER_TRACK)),
                        ),
                )
                .child(
                    step_button("+", format!("vu-{node_id}"), move |_, _, cx: &mut App| {
                        let (v, pid) = cx.try_global::<MixerGlobal>()
                            .and_then(|g| g.state.lock().unwrap().nodes.get(&node_id)
                                .map(|n| ((n.volume + 0.05).min(1.0), n.pulse_id)))
                            .unwrap_or((1.0, None));
                        if let Some(g) = cx.try_global::<MixerGlobal>() {
                            let _ = g.cmd_tx.send(EngineCommand::SetVolume {
                                node_id, volume: v,
                                pulse_id: pid, node_name: nn3.clone(), is_stream,
                            });
                        }
                    }),
                ),
        )
}

fn step_button(
    label: &str,
    id: String,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .w(px(24.)).h(px(24.))
        .flex().items_center().justify_center()
        .rounded_sm()
        .bg(rgb(BTN_BG))
        .text_color(rgb(TEXT_MUTED))
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(|s| s.bg(rgb(BTN_BG_HOVER)).text_color(rgb(TEXT)))
        .child(SharedString::from(label.to_string()))
        .id(SharedString::from(id))
        .on_click(on_click)
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() > max { s.chars().take(max).collect::<String>() + "…" } else { s.to_string() }
}
