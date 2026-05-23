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

// ═══════════════════════════════════════════════════════════════════════════════
//  DESIGN SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════

// ── Backgrounds ───────────────────────────────────────────────────────────────
const BG: u32             = 0x08090e;
const BG_ELEVATED: u32    = 0x0e1018;
const SURFACE: u32        = 0x13151f;
const CARD: u32           = 0x161924;
const CARD_HOVER: u32     = 0x1c1f2d;
const OVERLAY: u32        = 0x1e2130;

// ── Borders ───────────────────────────────────────────────────────────────────
const BORDER: u32         = 0x1e2130;
const BORDER_HOVER: u32   = 0x2a2e42;
const DIVIDER: u32        = 0x181b28;

// ── Text ──────────────────────────────────────────────────────────────────────
const TEXT_PRIMARY: u32   = 0xeef0f5;
const TEXT_SECONDARY: u32 = 0x8b90a5;
const TEXT_TERTIARY: u32  = 0x5a6075;

// ── Accents ───────────────────────────────────────────────────────────────────
const GREEN: u32          = 0x4ade80;
const GREEN_DIM: u32      = 0x1a2e26;
const BLUE: u32           = 0x60a5fa;
const BLUE_DIM: u32       = 0x1e2d4a;
const AMBER: u32          = 0xfbbf24;
const AMBER_DIM: u32      = 0x2d2510;
const PURPLE: u32         = 0xc084fc;
const PURPLE_DIM: u32     = 0x2a1f3a;
const RED: u32            = 0xf87171;
const RED_DIM: u32        = 0x3d1f24;

// ── Component tokens ──────────────────────────────────────────────────────────
const BTN_BG: u32         = 0x1e2232;
const BTN_BG_HOVER: u32   = 0x272b3d;
const SLIDER_TRACK: u32   = 0x252a3a;
const SLIDER_THUMB: u32   = 0xffffff;
const PILL_BG: u32        = 0x1a2340;
const PILL_BORDER: u32    = 0x2d3a66;
const ROUTE_TEXT: u32     = 0x93c5fd;
const TITLEBAR_H: f32     = 42.;

// ═══════════════════════════════════════════════════════════════════════════════
//  ICONS  (monochrome Unicode glyphs — not emojis)
// ═══════════════════════════════════════════════════════════════════════════════

fn ico_container(color: u32, symbol: &str) -> impl IntoElement {
    let dim = ((color & 0xFEFEFE) >> 1) | 0x080808;
    div()
        .w(px(20.)).h(px(20.))
        .flex().items_center().justify_center()
        .rounded_sm()
        .bg(rgb(dim))
        .child(
            div().text_color(rgb(color)).text_size(px(10.))
                .font_weight(gpui::FontWeight::BOLD)
                .child(SharedString::from(symbol.to_string()))
        )
}

fn ico_play(color: u32) -> impl IntoElement { ico_container(color, "▶") }
fn ico_circle(color: u32) -> impl IntoElement { ico_container(color, "●") }
fn ico_half_circle(color: u32) -> impl IntoElement { ico_container(color, "◐") }
fn ico_bullseye(color: u32) -> impl IntoElement { ico_container(color, "◉") }


// ═══════════════════════════════════════════════════════════════════════════════
//  RENDER
// ═══════════════════════════════════════════════════════════════════════════════

impl Render for MixerWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll_events(cx);
        _window.request_animation_frame();

        let (playback, app_inputs, inputs, outputs, out_list, out_names) =
            if let Some(g) = cx.try_global::<MixerGlobal>() {
                let s = g.state.lock().unwrap();
                let mut pb: Vec<u32> = s.playback_apps().iter().map(|n| n.id).collect();
                let mut ai: Vec<u32> = s.app_inputs().iter().map(|n| n.id).collect();
                let mut id: Vec<u32> = s.input_devices().iter().map(|n| n.id).collect();
                let mut od: Vec<u32> = s.output_devices().iter().map(|n| n.id).collect();
                pb.sort(); ai.sort(); id.sort(); od.sort();
                let ol: Vec<(u32, String)> = s.output_devices()
                    .iter().map(|n| (n.id, clip(&n.description, 24))).collect();
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
            .text_color(rgb(TEXT_PRIMARY))
            // ── custom titlebar ─────────────────────────────────────────
            .child(titlebar(_window, cx))
            // ── body ────────────────────────────────────────────────────
            .child(
                div()
                    .flex_1()
                    .id("body-scroll")
                    .overflow_y_scroll()
                    .px(px(32.)).py(px(28.))
                    .flex()
                    .flex_col()
                    .gap(px(40.))
                    .when(!playback.is_empty(), |d| {
                        d.child(render_section(
                            "Playback", ico_play(GREEN), GREEN, GREEN_DIM,
                            &playback, &out_list, &out_names,
                            &node_data, &link_map, open_dd, cx,
                        ))
                    })
                    .when(!app_inputs.is_empty(), |d| {
                        d.child(render_simple_section(
                            "App Inputs", ico_circle(PURPLE), PURPLE, PURPLE_DIM,
                            &app_inputs, &node_data, cx
                        ))
                    })
                    .when(!inputs.is_empty(), |d| {
                        d.child(render_simple_section(
                            "Input Devices", ico_half_circle(BLUE), BLUE, BLUE_DIM,
                            &inputs, &node_data, cx
                        ))
                    })
                    .when(!outputs.is_empty(), |d| {
                        d.child(render_simple_section(
                            "Output Devices", ico_bullseye(AMBER), AMBER, AMBER_DIM,
                            &outputs, &node_data, cx
                        ))
                    })
                    .when(
                        playback.is_empty() && app_inputs.is_empty()
                            && inputs.is_empty() && outputs.is_empty(),
                        |d| d.flex().items_center().justify_center().h_full().child(
                            div().flex().flex_col().items_center().gap(px(20.))
                                .child(
                                    div().w(px(56.)).h(px(56.))
                                        .flex().items_center().justify_center()
                                        .rounded_full()
                                        .bg(rgb(BG_ELEVATED))
                                        .border_1().border_color(rgb(BORDER))
                                        .child(div().text_size(px(24.)).text_color(rgb(TEXT_TERTIARY)).child("◉"))
                                )
                                .child(
                                    div().text_color(rgb(TEXT_SECONDARY)).text_size(px(14.))
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .child("Waiting for PipeWire…")
                                )
                                .child(
                                    div().text_color(rgb(TEXT_TERTIARY)).text_size(px(12.))
                                        .child("Audio nodes will appear here automatically")
                                ),
                        ),
                    ),
            )
            // ── resize handles ──────────────────────────────────────────
            .child(
                div()
                    .absolute().right_0().top_0().bottom_0()
                    .w(px(5.))
                    .cursor_ew_resize()
                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                        window.start_window_resize(ResizeEdge::Right);
                    }),
            )
            .child(
                div()
                    .absolute().left_0().right_0().bottom_0()
                    .h(px(5.))
                    .cursor_ns_resize()
                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                        window.start_window_resize(ResizeEdge::Bottom);
                    }),
            )
            .child(
                div()
                    .absolute().right_0().bottom_0()
                    .w(px(12.)).h(px(12.))
                    .cursor_nwse_resize()
                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                        window.start_window_resize(ResizeEdge::BottomRight);
                    }),
            )
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  TITLEBAR
// ═══════════════════════════════════════════════════════════════════════════════

fn titlebar(_window: &mut Window, _cx: &mut Context<MixerWindow>) -> impl IntoElement {
    div()
        .h(px(TITLEBAR_H))
        .px(px(18.))
        .flex().items_center().justify_between()
        .border_b_1().border_color(rgb(DIVIDER))
        .bg(rgb(BG_ELEVATED))
        // draggable region (left / center)
        .child(
            div()
                .flex().items_center().gap(px(12.))
                .flex_1()
                .h_full()
                .on_mouse_down(MouseButton::Left, |_, window, _| {
                    window.start_window_move();
                })
                .child(
                    div()
                        .w(px(26.)).h(px(26.))
                        .flex().items_center().justify_center()
                        .rounded_md()
                        .bg(rgb(GREEN_DIM))
                        .border_1().border_color(rgb(0x1a3a2a))
                        .child(
                            div().text_color(rgb(GREEN)).text_size(px(12.))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child("◉")
                        )
                )
                .child(
                    div()
                        .text_color(rgb(TEXT_PRIMARY))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(13.))
                        .child("MusicMixer")
                )
                .child(
                    div()
                        .px(px(6.)).py(px(2.))
                        .rounded_sm()
                        .bg(rgb(SURFACE))
                        .border_1().border_color(rgb(BORDER))
                        .text_color(rgb(TEXT_TERTIARY))
                        .text_size(px(9.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child("v0.1")
                ),
        )
        // window controls (right)
        .child(
            div().flex().items_center().gap(px(6.))
                .child(
                    div()
                        .w(px(30.)).h(px(30.))
                        .flex().items_center().justify_center()
                        .rounded_sm()
                        .text_color(rgb(TEXT_SECONDARY))
                        .text_size(px(16.))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(BTN_BG)).text_color(rgb(TEXT_PRIMARY)))
                        .child("−")
                        .id("titlebar-minimize")
                        .on_click(|_, window, _| {
                            window.minimize_window();
                        }),
                )
                .child(
                    div()
                        .w(px(30.)).h(px(30.))
                        .flex().items_center().justify_center()
                        .rounded_sm()
                        .text_color(rgb(TEXT_SECONDARY))
                        .text_size(px(16.))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(RED_DIM)).text_color(rgb(RED)))
                        .child("×")
                        .id("titlebar-close")
                        .on_click(|_, window, _| {
                            window.remove_window();
                        }),
                ),
        )
}

// ═══════════════════════════════════════════════════════════════════════════════
//  SECTIONS
// ═══════════════════════════════════════════════════════════════════════════════

fn render_section(
    title: &str,
    icon: impl IntoElement,
    color: u32,
    _dim: u32,
    ids: &[u32],
    out_list: &[(u32, String)],
    out_names: &std::collections::HashMap<u32, String>,
    node_data: &std::collections::HashMap<u32, (String, String, f32, bool, Option<u32>, bool)>,
    link_map: &[(u32, u32)],
    open_dd: Option<u32>,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let count = ids.len();
    let mut row = div().flex().flex_row().flex_wrap().gap(px(16.));
    for &node_id in ids {
        let (name, desc, volume, muted, pulse_id, is_stream) = node_data.get(&node_id)
            .cloned().unwrap_or_else(|| (format!("Node {node_id}"), String::new(), 1.0, false, None, true));
        let linked_outs: Vec<u32> = out_list.iter()
            .filter(|(oid, _)| link_map.iter().any(|(f, t)| *f == node_id && *t == *oid))
            .map(|(oid, _)| *oid).collect();
        let available: Vec<(u32, String)> = out_list.iter()
            .filter(|(oid, _)| !linked_outs.contains(oid)).cloned().collect();
        let card = routable_card(node_id, name, desc, volume, muted, pulse_id, is_stream,
            color, &linked_outs, out_list, out_names, &available, open_dd == Some(node_id), cx);
        row = row.child(card);
    }
    div().flex().flex_col().gap(px(16.))
        .child(section_header(title, icon, color, count))
        .child(row)
}

fn render_simple_section(
    title: &str,
    icon: impl IntoElement,
    color: u32,
    _dim: u32,
    ids: &[u32],
    node_data: &std::collections::HashMap<u32, (String, String, f32, bool, Option<u32>, bool)>,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let count = ids.len();
    let mut row = div().flex().flex_row().flex_wrap().gap(px(16.));
    for &node_id in ids {
        let (name, desc, volume, muted, pulse_id, is_stream) = node_data.get(&node_id)
            .cloned().unwrap_or_else(|| (format!("Node {node_id}"), String::new(), 1.0, false, None, false));
        row = row.child(simple_card(node_id, name, desc, volume, muted, pulse_id, is_stream, color, cx));
    }
    div().flex().flex_col().gap(px(16.))
        .child(section_header(title, icon, color, count))
        .child(row)
}

fn section_header(title: &str, icon: impl IntoElement, color: u32, count: usize) -> impl IntoElement {
    div().flex().items_center().gap(px(10.))
        .child(icon)
        .child(
            div()
                .text_color(rgb(color))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(11.))
                .child(SharedString::from(title.to_string())),
        )
        .child(
            div()
                .px(px(6.)).py(px(2.))
                .rounded_md()
                .bg(rgb(BG_ELEVATED))
                .border_1().border_color(rgb(BORDER))
                .text_color(rgb(TEXT_TERTIARY))
                .text_size(px(10.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(format!("{count}")),
        )
}

// ═══════════════════════════════════════════════════════════════════════════════
//  CARDS
// ═══════════════════════════════════════════════════════════════════════════════

fn routable_card(
    node_id: u32,
    name: String,
    desc: String,
    volume: f32,
    muted: bool,
    pulse_id: Option<u32>,
    is_stream: bool,
    color: u32,
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

    let pills: Vec<_> = linked.iter().map(|&out_id| {
        let label = all_out.iter().find(|(id, _)| *id == out_id)
            .map(|(_, n)| n.clone()).unwrap_or_else(|| format!("{out_id}"));
        let from_name = name.clone();
        let to_name = out_name_map.get(&out_id).cloned().unwrap_or_default();
        div()
            .flex().items_center().gap(px(5.))
            .h(px(26.))
            .px(px(9.))
            .rounded_md()
            .bg(rgb(PILL_BG))
            .border_1().border_color(rgb(PILL_BORDER))
            .child(
                div().text_color(rgb(ROUTE_TEXT)).text_size(px(11.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(SharedString::from(label))
            )
            .child(
                div()
                    .w(px(16.)).h(px(16.))
                    .flex().items_center().justify_center()
                    .rounded_sm()
                    .hover(|s| s.bg(rgb(0x2d3a5f)))
                    .text_color(rgb(ROUTE_TEXT)).text_size(px(11.))
                    .cursor_pointer()
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
            .px(px(12.)).py(px(8.))
            .cursor_pointer()
            .text_color(rgb(TEXT_PRIMARY)).text_size(px(12.))
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
        .w(px(280.)).flex().flex_col()
        .bg(rgb(CARD)).border_1().border_color(rgb(BORDER)).rounded_md()
        .overflow_hidden()
        .child(div().h(px(2.)).w_full().bg(rgb(color)))
        .hover(|s| s.border_color(rgb(BORDER_HOVER)).bg(rgb(CARD_HOVER)))
        .child(card_header(&name, &desc))
        .child(vol_controls(node_id, name.clone(), volume, muted, pulse_id, is_stream, color, cx))
        .child(
            div().px(px(16.)).pb(px(16.)).pt(px(4.)).flex().flex_col().gap(px(10.))
                .child(
                    div().flex().items_center().gap(px(6.))
                        .child(div().w(px(3.)).h(px(3.)).rounded_full().bg(rgb(TEXT_TERTIARY)))
                        .child(
                            div().text_color(rgb(TEXT_TERTIARY)).text_size(px(9.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("OUTPUT ROUTING"),
                        ),
                )
                .child(div().flex().flex_row().flex_wrap().gap(px(6.)).children(pills))
                .child(
                    div()
                        .flex().items_center().gap(px(5.))
                        .h(px(30.))
                        .px(px(10.))
                        .rounded_md()
                        .bg(rgb(PILL_BG)).border_1().border_color(rgb(PILL_BORDER))
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
                            .bg(rgb(OVERLAY)).border_1().border_color(rgb(BORDER)).rounded_md()
                            .children(dd_items),
                    )
                }),
        )
}

fn simple_card(
    node_id: u32,
    name: String,
    desc: String,
    volume: f32,
    muted: bool,
    pulse_id: Option<u32>,
    is_stream: bool,
    color: u32,
    cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    div()
        .w(px(280.)).flex().flex_col()
        .bg(rgb(CARD)).border_1().border_color(rgb(BORDER)).rounded_md()
        .overflow_hidden()
        .child(div().h(px(2.)).w_full().bg(rgb(color)))
        .hover(|s| s.border_color(rgb(BORDER_HOVER)).bg(rgb(CARD_HOVER)))
        .child(card_header(&name, &desc))
        .child(vol_controls(node_id, name, volume, muted, pulse_id, is_stream, color, cx))
}

fn card_header(name: &str, desc: &str) -> impl IntoElement {
    let primary = if desc.is_empty() { name } else { desc };
    let secondary = if desc.is_empty() { "" } else { name };
    div().px(px(16.)).pt(px(14.)).pb(px(8.)).flex().flex_col().gap(px(3.))
        .child(
            div().text_color(rgb(TEXT_PRIMARY)).font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(14.)).line_height(px(20.))
                .child(SharedString::from(clip(primary, 28))),
        )
        .when(!secondary.is_empty(), |d| d.child(
            div().text_color(rgb(TEXT_SECONDARY)).text_size(px(11.)).line_height(px(16.))
                .child(SharedString::from(clip(secondary, 32))),
        ))
}

// ═══════════════════════════════════════════════════════════════════════════════
//  VOLUME CONTROLS
// ═══════════════════════════════════════════════════════════════════════════════

fn vol_controls(
    node_id: u32,
    node_name: String,
    volume: f32,
    muted: bool,
    #[allow(unused_variables)]
    pulse_id: Option<u32>,
    is_stream: bool,
    accent: u32,
    _cx: &mut Context<MixerWindow>,
) -> impl IntoElement {
    let vol_pct = (volume * 100.0).round() as u32;
    let track_w = 130.0;
    let thumb_w = 14.0;
    let travel = track_w - thumb_w; // 106 — thumb stays inside track
    let fill_w = px(volume * track_w);
    let thumb_left = px(volume * travel);
    let nn1 = node_name.clone();
    let nn2 = node_name.clone();
    let nn3 = node_name.clone();

    let bar_color = accent;
    let mute_bg = if muted { RED_DIM } else { BTN_BG };
    let mute_text = if muted { RED } else { TEXT_SECONDARY };
    let mute_label = if muted { "Unmute" } else { "Mute" };

    div().px(px(16.)).pb(px(16.)).flex().flex_col().gap(px(10.))
        // row: mute btn + percentage
        .child(
            div().flex().items_center().justify_between()
                .child(
                    div()
                        .h(px(28.))
                        .px(px(10.))
                        .flex().items_center()
                        .rounded_md()
                        .bg(rgb(mute_bg))
                        .border_1().border_color(rgb(if muted { RED_DIM } else { BORDER }))
                        .text_color(rgb(mute_text))
                        .text_size(px(10.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .cursor_pointer()
                        .hover(|s| if muted {
                            s.bg(rgb(0x4a2028)).border_color(rgb(0x5a3038))
                        } else {
                            s.bg(rgb(BTN_BG_HOVER)).border_color(rgb(BORDER_HOVER))
                        })
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
                .child(
                    div().text_color(rgb(TEXT_SECONDARY)).text_size(px(12.))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(format!("{vol_pct}%"))
                ),
        )
        // slider row
        .child(
            div().flex().items_center().gap(px(10.))
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
                    div().w(px(track_w)).h(px(6.)).rounded_full().bg(rgb(SLIDER_TRACK))
                        .relative()
                        .child(
                            div().absolute().left_0().top_0().bottom_0()
                                .w(fill_w)
                                .rounded_full()
                                .bg(rgb(bar_color)),
                        )
                        .child(
                            div().absolute().top(px(-4.))
                                .left(thumb_left)
                                .w(px(thumb_w)).h(px(thumb_w))
                                .rounded_full()
                                .bg(rgb(SLIDER_THUMB))
                                .border_1().border_color(rgb(SLIDER_TRACK))
                                .shadow_md(),
                        )
                        .child(
                            div().absolute().inset_0().rounded_full().cursor_pointer()
                                .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                    let mouse_x: f32 = event.position.x.into();
                                    let body_pad = 32.0;
                                    let card_w = 280.0;
                                    let gap = 16.0;
                                    let col = ((mouse_x - body_pad) / (card_w + gap)).floor().max(0.0);
                                    let card_left = body_pad + col * (card_w + gap);
                                    let track_left = card_left + 16.0 + 28.0 + 10.0;
                                    let rel_x = mouse_x - track_left;
                                    let new_volume = (rel_x / track_w).clamp(0.0, 1.0);
                                    if let Some(g) = cx.try_global::<MixerGlobal>() {
                                        let pid = g.state.lock().unwrap().nodes.get(&node_id)
                                            .map(|n| n.pulse_id).unwrap_or(None);
                                        let _ = g.cmd_tx.send(EngineCommand::SetVolume {
                                            node_id, volume: new_volume,
                                            pulse_id: pid, node_name: node_name.clone(), is_stream,
                                        });
                                    }
                                }),
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
        .w(px(28.)).h(px(28.))
        .flex().items_center().justify_center()
        .rounded_md()
        .bg(rgb(BTN_BG))
        .border_1().border_color(rgb(BORDER))
        .text_color(rgb(TEXT_SECONDARY))
        .text_size(px(13.))
        .font_weight(gpui::FontWeight::BOLD)
        .cursor_pointer()
        .hover(|s| s.bg(rgb(BTN_BG_HOVER)).border_color(rgb(BORDER_HOVER)).text_color(rgb(TEXT_PRIMARY)))
        .child(SharedString::from(label.to_string()))
        .id(SharedString::from(id))
        .on_click(on_click)
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() > max { s.chars().take(max).collect::<String>() + "…" } else { s.to_string() }
}
