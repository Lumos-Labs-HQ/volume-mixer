use gpui::{
    div, linear_color_stop, linear_gradient, prelude::*, px, rgb, App, Context, IntoElement,
    MouseButton, Render, ResizeEdge, SharedString, Window,
};

use crate::models::{AppGroup, EngineCommand, EngineEvent};
use crate::state::MixerGlobal;
use crate::ui::icons;

pub struct MixerWindow {
    open_output_popover: Option<u32>,
    open_input_dropdown: Option<u32>,
}

impl MixerWindow {
    pub fn new() -> Self {
        MixerWindow {
            open_output_popover: None,
            open_input_dropdown: None,
        }
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

// ═══════════════════════════════════════════════════════════════════════════════
//  DESIGN SYSTEM (matches TSX example)
// ═══════════════════════════════════════════════════════════════════════════════

// ── Backgrounds ───────────────────────────────────────────────────────────────
const BG_FROM: u32 = 0x0a0e1a;
const BG_TO: u32   = 0x0d1117;
const CARD: u32    = 0x0f1219;
const CARD_HOVER: u32 = 0x131721;
const OVERLAY: u32 = 0x161b22;

// ── Text ──────────────────────────────────────────────────────────────────────
const TEXT_PRIMARY: u32   = 0xe2e8f0;
const TEXT_SECONDARY: u32 = 0x94a3b8;
const TEXT_MUTED: u32     = 0x64748b;

// ── Accents ───────────────────────────────────────────────────────────────────
const VIOLET_400: u32  = 0xa78bfa;
const VIOLET_500: u32  = 0x8b5cf6;
const FUCHSIA_500: u32 = 0xd946ef;
const EMERALD_400: u32 = 0x34d399;
const EMERALD_500: u32 = 0x10b981;
const TEAL_500: u32    = 0x14b8a6;
const AMBER_400: u32   = 0xfbbf24;
const AMBER_500: u32   = 0xf59e0b;
const ORANGE_500: u32  = 0xf97316;
const ROSE_400: u32    = 0xfb7185;
const ROSE_500: u32    = 0xf43f5e;

// ── Component tokens ──────────────────────────────────────────────────────────
const SLIDER_TRACK: u32 = 0xffffff;
const TITLEBAR_H: f32   = 42.;

/// Color with alpha helper (GPUI rgba takes 0xRRGGBBAA)
fn ca(hex: u32, alpha: f32) -> gpui::Rgba {
    gpui::Rgba {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: alpha,
    }
}

fn bg_gradient() -> gpui::Background {
    linear_gradient(
        135.0,
        linear_color_stop(rgb(BG_FROM), 0.0),
        linear_color_stop(rgb(BG_TO), 1.0),
    )
}

fn app_icon_bg() -> gpui::Background {
    linear_gradient(
        135.0,
        linear_color_stop(ca(VIOLET_500, 0.12), 0.0),
        linear_color_stop(ca(FUCHSIA_500, 0.12), 1.0),
    )
}

fn output_icon_bg() -> gpui::Background {
    linear_gradient(
        135.0,
        linear_color_stop(ca(EMERALD_500, 0.12), 0.0),
        linear_color_stop(ca(TEAL_500, 0.12), 1.0),
    )
}

fn input_icon_bg() -> gpui::Background {
    linear_gradient(
        135.0,
        linear_color_stop(ca(AMBER_500, 0.12), 0.0),
        linear_color_stop(ca(ORANGE_500, 0.12), 1.0),
    )
}

fn slider_gradient() -> gpui::Background {
    linear_gradient(
        90.0,
        linear_color_stop(ca(VIOLET_500, 1.0), 0.0),
        linear_color_stop(ca(FUCHSIA_500, 1.0), 1.0),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
//  RENDER
// ═══════════════════════════════════════════════════════════════════════════════

impl Render for MixerWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll_events(cx);
        _window.request_animation_frame();

        // Collect all data under one lock to avoid borrow issues
        let (app_groups, out_devices, in_devices, empty) = if let Some(g) = cx.try_global::<MixerGlobal>() {
            let s = g.state.lock().unwrap();
            let apps = s.app_groups();
            let outs: Vec<(u32, String, String)> = s.output_devices()
                .iter().map(|n| (n.id, n.description.clone(), n.name.clone())).collect();
            let ins: Vec<(u32, String, String)> = s.input_devices()
                .iter().map(|n| (n.id, n.description.clone(), n.name.clone())).collect();
            let e = apps.is_empty() && outs.is_empty() && ins.is_empty();
            (apps, outs, ins, e)
        } else {
            (vec![], vec![], vec![], true)
        };

        let app_count = app_groups.len();
        let out_count = out_devices.len();
        let in_count = in_devices.len();
        let open_out_popover = self.open_output_popover;
        let open_in_dropdown = self.open_input_dropdown;

        // Build children collections before chaining to avoid closure borrow issues
        let mut app_cards: Vec<gpui::Div> = vec![];
        for group in &app_groups {
            app_cards.push(render_app_card(
                group,
                &out_devices,
                &in_devices,
                open_out_popover,
                open_in_dropdown,
                cx,
            ));
        }

        let mut out_rows: Vec<gpui::Div> = vec![];
        for (id, desc, name) in &out_devices {
            out_rows.push(render_device_row(*id, desc.clone(), name.clone(), "output", cx));
        }

        let mut in_rows: Vec<gpui::Div> = vec![];
        for (id, desc, name) in &in_devices {
            in_rows.push(render_device_row(*id, desc.clone(), name.clone(), "input", cx));
        }

        let mut body_children: Vec<gpui::Div> = vec![];
        body_children.push(render_header(app_count, out_count, in_count));
        if !app_cards.is_empty() {
            body_children.push(render_applications_section(app_cards));
        }
        if !out_rows.is_empty() {
            body_children.push(render_output_devices_section(out_rows));
        }
        if !in_rows.is_empty() {
            body_children.push(render_input_devices_section(in_rows));
        }
        if empty {
            body_children.push(empty_state());
        }

        let mut body = div()
            .w_full()
            .max_w(px(1024.))
            .mx_auto()
            .flex().flex_col().gap(px(24.));
        for child in body_children {
            body = body.child(child);
        }

        div()
            .size_full()
            .relative()
            .flex().flex_col()
            .bg(bg_gradient())
            .text_color(rgb(TEXT_PRIMARY))
            .font_family("Inter")
            .child(titlebar(_window, cx))
            .child(
                div()
                    .flex_1()
                    .id("body-scroll")
                    .overflow_y_scroll()
                    .px(px(24.)).py(px(24.))
                    .flex()
                    .flex_col()
                    .child(body)
            )
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
        .border_b_1().border_color(rgb(0x1a1f2e))
        .bg(rgb(BG_TO))
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
                        .bg(app_icon_bg())
                        .border_1().border_color(ca(VIOLET_500, 0.15))
                        .child(icons::ico_music(12., VIOLET_400, None))
                )
                .child(
                    div()
                        .text_color(rgb(TEXT_PRIMARY))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_size(px(13.))
                        .child("PipeWire Mixer")
                ),
        )
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
                        .hover(|s| s.bg(rgb(0x1e2330)).text_color(rgb(TEXT_PRIMARY)))
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
                        .hover(|s| s.bg(ca(ROSE_500, 0.15)).text_color(rgb(ROSE_400)))
                        .child("×")
                        .id("titlebar-close")
                        .on_click(|_, window, _| {
                            window.remove_window();
                        }),
                ),
        )
}

// ═══════════════════════════════════════════════════════════════════════════════
//  HEADER
// ═══════════════════════════════════════════════════════════════════════════════

fn render_header(apps: usize, outs: usize, ins: usize) -> gpui::Div {
    div().flex().flex_col().gap(px(4.))
        .child(
            div()
                .text_size(px(22.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(VIOLET_400))
                .child("PipeWire Mixer")
        )
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(TEXT_MUTED))
                .child(format!("{apps} applications • {outs} outputs • {ins} inputs"))
        )
}

fn empty_state() -> gpui::Div {
    div().flex().items_center().justify_center().h_full().child(
        div().flex().flex_col().items_center().gap(px(20.))
            .child(
                div().w(px(56.)).h(px(56.))
                    .flex().items_center().justify_center()
                    .rounded_full()
                    .bg(rgb(0x1a1f2e))
                    .border_1().border_color(rgb(0x1e2330))
                    .child(icons::ico_monitor(24., TEXT_MUTED, None))
            )
            .child(
                div().text_color(rgb(TEXT_SECONDARY)).text_size(px(14.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child("Waiting for PipeWire…")
            )
            .child(
                div().text_color(rgb(TEXT_MUTED)).text_size(px(12.))
                    .child("Audio nodes will appear here automatically")
            ),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
//  APPLICATIONS SECTION
// ═══════════════════════════════════════════════════════════════════════════════

fn render_applications_section(cards: Vec<gpui::Div>) -> gpui::Div {
    let mut section = div().flex().flex_col().gap(px(12.));
    section = section.child(
        div()
            .text_size(px(12.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT_SECONDARY))
            .px(px(4.))
            .child("Applications")
    );
    let mut list = div().flex().flex_col().gap(px(10.));
    for card in cards {
        list = list.child(card);
    }
    section.child(list)
}

fn render_app_card(
    group: &AppGroup,
    output_devices: &[(u32, String, String)],
    input_devices: &[(u32, String, String)],
    open_out_popover: Option<u32>,
    open_in_dropdown: Option<u32>,
    cx: &mut Context<MixerWindow>,
) -> gpui::Div {
    let group = group.clone();

    let linked_outputs: Vec<(u32, String)> = if let Some(g) = cx.try_global::<MixerGlobal>() {
        let s = g.state.lock().unwrap();
        if let Some(pid) = group.playback_id {
            s.playback_output_devices(pid)
                .into_iter()
                .filter_map(|id| s.nodes.get(&id).map(|n| (id, n.description.clone())))
                .collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let current_input: Option<(u32, String)> = if let Some(g) = cx.try_global::<MixerGlobal>() {
        let s = g.state.lock().unwrap();
        group.capture_id
            .and_then(|cid| s.capture_input_device(cid))
            .and_then(|id| s.nodes.get(&id).map(|n| (id, n.description.clone())))
    } else {
        None
    };

    let out_count = linked_outputs.len();
    let in_count = if current_input.is_some() { 1 } else { 0 };

    let vol_node_id = group.playback_id.unwrap_or(group.capture_id.unwrap_or(0));
    let vol_node_name = group.name.clone();
    let is_stream = true;

    let output_routing = render_output_routing(
        &group,
        output_devices,
        &linked_outputs,
        open_out_popover,
        cx,
    );

    let input_routing = render_input_routing(
        &group,
        input_devices,
        current_input,
        open_in_dropdown,
        cx,
    );

    div()
        .w_full()
        .rounded_2xl()
        .bg(rgb(CARD))
        .p(px(16.))
        .flex().flex_col().gap(px(12.))
        .hover(|s| s.bg(rgb(CARD_HOVER)))
        .child(
            div().flex().items_center().gap(px(14.))
                .child(
                    div()
                        .w(px(40.)).h(px(40.))
                        .flex().items_center().justify_center()
                        .rounded_xl()
                        .bg(app_icon_bg())
                        .child(icons::app_icon(group.icon, 20., VIOLET_400, None))
                )
                .child(
                    div()
                        .w(px(200.))
                        .flex().flex_col().gap(px(2.))
                        .child(
                            div()
                                .text_size(px(13.))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(rgb(TEXT_PRIMARY))
                                .child(clip(&group.name, 32))
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(TEXT_MUTED))
                                .child(format!("{out_count} out • {in_count} in"))
                        )
                )
                .child(
                    volume_slider(vol_node_id, vol_node_name, group.volume, group.muted, is_stream)
                )
        )
        .child(
            div().flex().flex_row().gap(px(16.))
                .child(output_routing)
                .child(input_routing)
        )
}

// ═══════════════════════════════════════════════════════════════════════════════
//  OUTPUT ROUTING
// ═══════════════════════════════════════════════════════════════════════════════

fn render_output_routing(
    group: &AppGroup,
    output_devices: &[(u32, String, String)],
    linked_outputs: &[(u32, String)],
    open_popover: Option<u32>,
    cx: &mut Context<MixerWindow>,
) -> gpui::Div {
    let group_name = group.name.clone();
    let group_id = group.playback_id.unwrap_or(0);
    let is_open = open_popover == Some(group_id);

    let available: Vec<(u32, String, String)> = output_devices.iter()
        .filter(|(id, _, _)| !linked_outputs.iter().any(|(lid, _)| *lid == *id))
        .cloned()
        .collect();

    let mut pills_row = div().flex().flex_row().flex_wrap().items_center().gap(px(6.));

    for (out_id, out_desc) in linked_outputs {
        let from_name = group_name.clone();
        let to_name = output_devices.iter()
            .find(|(id, _, _)| *id == *out_id)
            .map(|(_, _, name)| name.clone())
            .unwrap_or_default();
        let desc = out_desc.clone();
        let device_type = infer_device_type(&desc);

        let pill = div()
            .flex().items_center().gap(px(6.))
            .h(px(28.))
            .px(px(10.))
            .rounded_full()
            .bg(ca(EMERALD_500, 0.10))
            .text_color(rgb(EMERALD_400))
            .child(icons::output_device_icon(&device_type, 14., EMERALD_400, None))
            .child(
                div()
                    .text_size(px(11.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(clip(&desc, 24))
            )
            .child(
                div()
                    .w(px(18.)).h(px(18.))
                    .flex().items_center().justify_center()
                    .rounded_full()
                    .cursor_pointer()
                    .hover(|s| s.bg(ca(EMERALD_500, 0.15)))
                    .child(icons::ico_x(12., EMERALD_400))
                    .id(SharedString::from(format!("unroute-{}", out_id)))
                    .on_click(move |_, _, cx: &mut App| {
                        if let Some(g) = cx.try_global::<MixerGlobal>() {
                            let _ = g.cmd_tx.send(EngineCommand::RemoveLink {
                                from_name: from_name.clone(),
                                to_name: to_name.clone(),
                            });
                        }
                    })
            );
        pills_row = pills_row.child(pill);
    }

    let add_btn = div()
        .flex().items_center().gap(px(4.))
        .h(px(28.))
        .px(px(10.))
        .rounded_full()
        .bg(rgb(0x1a1f2e))
        .border_1().border_color(rgb(0x252a3a))
        .cursor_pointer()
        .text_color(rgb(TEXT_MUTED))
        .hover(|s| s.bg(rgb(0x1e2330)).text_color(rgb(TEXT_SECONDARY)))
        .child(icons::ico_plus(12., TEXT_MUTED))
        .child(
            div().text_size(px(11.)).font_weight(gpui::FontWeight::MEDIUM).child("Add")
        )
        .id(SharedString::from(format!("add-out-{group_id}")))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.open_output_popover = if this.open_output_popover == Some(group_id) { None } else { Some(group_id) };
            cx.notify();
        }));

    pills_row = pills_row.child(add_btn);

    if is_open && !available.is_empty() {
        let mut popover = div()
            .bg(rgb(OVERLAY))
            .border_1().border_color(rgb(0x252a3a))
            .rounded_xl()
            .p(px(6.))
            .w(px(220.))
            .flex().flex_col().gap(px(2.));

        for (out_id, out_desc, out_name) in available {
            let from_name = group_name.clone();
            let to_name = out_name.clone();
            let desc = out_desc.clone();
            let device_type = infer_device_type(&desc);
            let item = div()
                .w_full()
                .flex().items_center().gap(px(8.))
                .px(px(10.)).py(px(8.))
                .rounded_xl()
                .cursor_pointer()
                .text_color(rgb(TEXT_SECONDARY))
                .hover(|s| s.bg(rgb(0x1e2330)))
                .child(icons::output_device_icon(&device_type, 14., TEXT_SECONDARY, None))
                .child(
                    div().text_size(px(12.)).child(clip(&desc, 28))
                )
                .id(SharedString::from(format!("out-item-{group_id}-{out_id}")))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(g) = cx.try_global::<MixerGlobal>() {
                        let _ = g.cmd_tx.send(EngineCommand::CreateLink {
                            from_name: from_name.clone(),
                            to_name: to_name.clone(),
                        });
                    }
                    this.open_output_popover = None;
                    cx.notify();
                }));
            popover = popover.child(item);
        }
        pills_row = pills_row.child(popover);
    }

    div()
        .flex_1()
        .flex().flex_col().gap(px(8.))
        .child(
            div()
                .text_size(px(10.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child("OUTPUT")
        )
        .child(pills_row)
}

// ═══════════════════════════════════════════════════════════════════════════════
//  INPUT ROUTING
// ═══════════════════════════════════════════════════════════════════════════════

fn render_input_routing(
    group: &AppGroup,
    input_devices: &[(u32, String, String)],
    current_input: Option<(u32, String)>,
    open_dropdown: Option<u32>,
    cx: &mut Context<MixerWindow>,
) -> gpui::Div {
    let group_id = group.capture_id.unwrap_or(0);
    let is_open = open_dropdown == Some(group_id);
    let capture_id = group.capture_id;

    let app_input_name = if let Some(g) = cx.try_global::<MixerGlobal>() {
        let s = g.state.lock().unwrap();
        group.capture_id
            .and_then(|id| s.nodes.get(&id).map(|n| n.name.clone()))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let current_input_name = current_input.as_ref().map(|(_, n)| n.clone()).unwrap_or_else(|| "None".to_string());

    let trigger = div()
        .flex().items_center().justify_between()
        .gap(px(6.))
        .h(px(28.))
        .px(px(10.))
        .rounded_full()
        .bg(ca(AMBER_500, 0.10))
        .text_color(rgb(AMBER_400))
        .cursor_pointer()
        .hover(|s| s.bg(ca(AMBER_500, 0.15)))
        .child(
            div().flex().items_center().gap(px(6.)).min_w_0()
                .child(icons::ico_mic2(14., AMBER_400, None))
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(clip(&current_input_name, 20))
                )
        )
        .child(icons::ico_chevron_down(12., AMBER_400))
        .id(SharedString::from(format!("in-trigger-{group_id}")))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.open_input_dropdown = if this.open_input_dropdown == Some(group_id) { None } else { Some(group_id) };
            cx.notify();
        }));

    let mut dropdown = div()
        .bg(rgb(OVERLAY))
        .border_1().border_color(rgb(0x252a3a))
        .rounded_xl()
        .p(px(6.))
        .w(px(220.))
        .flex().flex_col().gap(px(2.));

    // None option
    let app_input_name_for_none = app_input_name.clone();
    let none_item = div()
        .w_full()
        .flex().items_center().gap(px(8.))
        .px(px(10.)).py(px(8.))
        .rounded_xl()
        .cursor_pointer()
        .text_color(rgb(TEXT_SECONDARY))
        .hover(|s| s.bg(rgb(0x1e2330)))
        .child(
            div().text_size(px(12.)).child("None")
        )
        .id(SharedString::from(format!("in-none-{group_id}")))
        .on_click(cx.listener(move |this, _, _, cx| {
            if let Some(g) = cx.try_global::<MixerGlobal>() {
                let s = g.state.lock().unwrap();
                if let Some(cid) = capture_id {
                    if let Some(old_input_id) = s.capture_input_device(cid) {
                        if let Some(old_input) = s.nodes.get(&old_input_id) {
                            let _ = g.cmd_tx.send(EngineCommand::RemoveLink {
                                from_name: old_input.name.clone(),
                                to_name: app_input_name_for_none.clone(),
                            });
                        }
                    }
                }
            }
            this.open_input_dropdown = None;
            cx.notify();
        }));
    dropdown = dropdown.child(none_item);

    for (device_id, device_desc, device_name) in input_devices {
        let device_name = device_name.clone();
        let device_desc = device_desc.clone();
        let capture_id = group.capture_id;
        let app_input_name = app_input_name.clone();
        let item = div()
            .w_full()
            .flex().items_center().gap(px(8.))
            .px(px(10.)).py(px(8.))
            .rounded_xl()
            .cursor_pointer()
            .text_color(rgb(TEXT_SECONDARY))
            .hover(|s| s.bg(rgb(0x1e2330)))
            .child(icons::ico_mic2(14., TEXT_SECONDARY, None))
            .child(
                div().text_size(px(12.)).child(clip(&device_desc, 28))
            )
            .id(SharedString::from(format!("in-item-{group_id}-{device_id}")))
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(g) = cx.try_global::<MixerGlobal>() {
                    let s = g.state.lock().unwrap();
                    // Remove existing link first
                    if let Some(cid) = capture_id {
                        if let Some(old_input_id) = s.capture_input_device(cid) {
                            if let Some(old_input) = s.nodes.get(&old_input_id) {
                                let _ = g.cmd_tx.send(EngineCommand::RemoveLink {
                                    from_name: old_input.name.clone(),
                                    to_name: app_input_name.clone(),
                                });
                            }
                        }
                    }
                    // Create new link
                    let _ = g.cmd_tx.send(EngineCommand::CreateLink {
                        from_name: device_name.clone(),
                        to_name: app_input_name.clone(),
                    });
                }
                this.open_input_dropdown = None;
                cx.notify();
            }));
        dropdown = dropdown.child(item);
    }

    div()
        .flex_1()
        .flex().flex_col().gap(px(8.))
        .child(
            div()
                .text_size(px(10.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT_MUTED))
                .child("INPUT")
        )
        .child(
            div().relative()
                .child(trigger)
                .when(is_open, |d| d.child(dropdown))
        )
}

// ═══════════════════════════════════════════════════════════════════════════════
//  OUTPUT DEVICES SECTION
// ═══════════════════════════════════════════════════════════════════════════════

fn render_output_devices_section(rows: Vec<gpui::Div>) -> gpui::Div {
    let mut section = div().flex().flex_col().gap(px(12.));
    section = section.child(
        div()
            .text_size(px(12.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT_SECONDARY))
            .px(px(4.))
            .child("Output Devices")
    );
    let mut list = div().flex().flex_col().gap(px(8.));
    for row in rows {
        list = list.child(row);
    }
    section.child(list)
}

// ═══════════════════════════════════════════════════════════════════════════════
//  INPUT DEVICES SECTION
// ═══════════════════════════════════════════════════════════════════════════════

fn render_input_devices_section(rows: Vec<gpui::Div>) -> gpui::Div {
    let mut section = div().flex().flex_col().gap(px(12.));
    section = section.child(
        div()
            .text_size(px(12.))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT_SECONDARY))
            .px(px(4.))
            .child("Input Devices")
    );
    let mut list = div().flex().flex_col().gap(px(8.));
    for row in rows {
        list = list.child(row);
    }
    section.child(list)
}

// ═══════════════════════════════════════════════════════════════════════════════
//  DEVICE ROW (shared for output/input devices)
// ═══════════════════════════════════════════════════════════════════════════════

fn render_device_row(
    node_id: u32,
    description: String,
    node_name: String,
    kind: &str,
    cx: &mut Context<MixerWindow>,
) -> gpui::Div {
    let volume = if let Some(g) = cx.try_global::<MixerGlobal>() {
        g.state.lock().unwrap().nodes.get(&node_id).map(|n| n.volume).unwrap_or(1.0)
    } else { 1.0 };
    let muted = if let Some(g) = cx.try_global::<MixerGlobal>() {
        g.state.lock().unwrap().nodes.get(&node_id).map(|n| n.muted).unwrap_or(false)
    } else { false };
    let is_stream = false;

    let (icon, icon_bg) = if kind == "input" {
        (icons::ico_mic2(20., AMBER_400, None), input_icon_bg())
    } else {
        let device_type = infer_device_type(&description);
        let ico = match device_type.as_str() {
            "headphones" => icons::ico_headphones(20., EMERALD_400, None),
            "monitor" => icons::ico_monitor(20., EMERALD_400, None),
            _ => icons::ico_speaker(20., EMERALD_400, None),
        };
        (ico, output_icon_bg())
    };

    div()
        .w_full()
        .rounded_2xl()
        .bg(rgb(CARD))
        .p(px(14.))
        .flex().items_center().gap(px(14.))
        .hover(|s| s.bg(rgb(CARD_HOVER)))
        .child(
            div()
                .w(px(40.)).h(px(40.))
                .flex().items_center().justify_center()
                .rounded_xl()
                .bg(icon_bg)
                .child(icon)
        )
        .child(
            div()
                .w(px(200.))
                .flex().flex_col().gap(px(2.))
                .child(
                    div()
                        .text_size(px(13.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(rgb(TEXT_PRIMARY))
                        .child(clip(&description, 32))
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(if kind == "input" { "Input" } else { "Output" })
                )
        )
        .child(
            volume_slider(node_id, node_name, volume, muted, is_stream)
        )
}

// ═══════════════════════════════════════════════════════════════════════════════
//  VOLUME SLIDER
// ═══════════════════════════════════════════════════════════════════════════════

fn volume_slider(
    node_id: u32,
    node_name: String,
    volume: f32,
    muted: bool,
    is_stream: bool,
) -> gpui::Div {
    let vol_pct = if muted { 0 } else { (volume * 100.0).round() as u32 };
    let track_w = 200.0;
    let thumb_w = 14.0;
    let travel = track_w - thumb_w;
    let fill_w = px(volume * track_w);
    let thumb_left = px(volume * travel);
    let nn = node_name.clone();
    let nn1 = node_name.clone();
    let nn2 = node_name.clone();

    let mute_color = if muted { ROSE_400 } else { TEXT_SECONDARY };
    let mute_bg = if muted { ca(ROSE_500, 0.15) } else { rgb(0x1a1f2e) };
    let mute_hover = if muted { ca(ROSE_500, 0.25) } else { rgb(0x1e2330) };

    let mute_btn = div()
        .w(px(32.)).h(px(32.))
        .flex().items_center().justify_center()
        .rounded_full()
        .bg(mute_bg)
        .cursor_pointer()
        .hover(|s| s.bg(mute_hover))
        .child(
            if muted {
                icons::ico_volume_off(16., mute_color)
            } else {
                icons::ico_volume_on(16., mute_color)
            }
        )
        .id(SharedString::from(format!("mute-{node_id}")))
        .on_click(move |_, _, cx: &mut App| {
            let (new_muted, pid) = cx.try_global::<MixerGlobal>()
                .and_then(|g| g.state.lock().unwrap().nodes.get(&node_id)
                    .map(|n| (!n.muted, n.pulse_id)))
                .unwrap_or((false, None));
            if let Some(g) = cx.try_global::<MixerGlobal>() {
                let _ = g.cmd_tx.send(EngineCommand::SetMute {
                    node_id, muted: new_muted,
                    pulse_id: pid, node_name: nn.clone(), is_stream,
                });
            }
        });

    let slider_track = div()
        .w(px(track_w)).h(px(6.))
        .rounded_full()
        .bg(ca(SLIDER_TRACK, 0.05))
        .relative()
        .child(
            div().absolute().left_0().top_0().bottom_0()
                .w(fill_w)
                .rounded_full()
                .bg(slider_gradient()),
        )
        .child(
            div().absolute().top(px(-4.))
                .left(thumb_left)
                .w(px(thumb_w)).h(px(thumb_w))
                .rounded_full()
                .bg(rgb(0xffffff))
                .border_1().border_color(ca(SLIDER_TRACK, 0.10))
                .shadow_md(),
        );

    let pct_label = div()
        .w(px(40.))
        .text_right()
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(rgb(TEXT_SECONDARY))
        .child(format!("{vol_pct}%"));

    let dec_btn = step_button("−", move |_, _, cx: &mut App| {
        let (v, pid) = cx.try_global::<MixerGlobal>()
            .and_then(|g| g.state.lock().unwrap().nodes.get(&node_id)
                .map(|n| ((n.volume - 0.05).max(0.0), n.pulse_id)))
            .unwrap_or((0.0, None));
        if let Some(g) = cx.try_global::<MixerGlobal>() {
            let _ = g.cmd_tx.send(EngineCommand::SetVolume {
                node_id, volume: v,
                pulse_id: pid, node_name: nn1.clone(), is_stream,
            });
        }
    });

    let inc_btn = step_button("+", move |_, _, cx: &mut App| {
        let (v, pid) = cx.try_global::<MixerGlobal>()
            .and_then(|g| g.state.lock().unwrap().nodes.get(&node_id)
                .map(|n| ((n.volume + 0.05).min(1.0), n.pulse_id)))
            .unwrap_or((1.0, None));
        if let Some(g) = cx.try_global::<MixerGlobal>() {
            let _ = g.cmd_tx.send(EngineCommand::SetVolume {
                node_id, volume: v,
                pulse_id: pid, node_name: nn2.clone(), is_stream,
            });
        }
    });

    div()
        .flex_1()
        .flex().items_center().gap(px(10.))
        .child(mute_btn)
        .child(slider_track)
        .child(pct_label)
        .child(dec_btn)
        .child(inc_btn)
}

fn step_button(
    label: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .w(px(24.)).h(px(24.))
        .flex().items_center().justify_center()
        .rounded_md()
        .bg(rgb(0x1a1f2e))
        .border_1().border_color(rgb(0x252a3a))
        .text_color(rgb(TEXT_MUTED))
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::BOLD)
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x1e2330)).border_color(rgb(0x2a2e42)).text_color(rgb(TEXT_SECONDARY)))
        .child(SharedString::from(label.to_string()))
        .id(SharedString::from(format!("step-{label}")))
        .on_click(on_click)
}

// ═══════════════════════════════════════════════════════════════════════════════
//  HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() > max { s.chars().take(max).collect::<String>() + "…" } else { s.to_string() }
}

fn infer_device_type(description: &str) -> String {
    let lower = description.to_lowercase();
    if lower.contains("headphone") || lower.contains("headset") {
        "headphones".to_string()
    } else if lower.contains("hdmi") || lower.contains("display") || lower.contains("monitor") {
        "monitor".to_string()
    } else {
        "speakers".to_string()
    }
}
