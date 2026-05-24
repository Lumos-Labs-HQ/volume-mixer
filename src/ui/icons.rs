use gpui::{div, prelude::*, px, rgb, svg, Div, SharedString};

fn icon_container(path: &str, size: f32, color: u32, bg_color: Option<u32>) -> Div {
    let path_str = SharedString::from(path.to_string());
    let icon = svg()
        .path(path_str)
        .size(px(size))
        .text_color(rgb(color));

    match bg_color {
        Some(bg) => div()
            .w(px(size + 16.)).h(px(size + 16.))
            .flex().items_center().justify_center()
            .rounded_xl()
            .bg(rgb(bg))
            .child(icon),
        None => div()
            .flex().items_center().justify_center()
            .child(icon),
    }
}

// ── App icons (violet/fuchsia theme) ─────────────────────────────────────────
pub fn ico_music(size: f32, color: u32, bg: Option<u32>) -> Div {
    icon_container("icons/music.svg", size, color, bg)
}

pub fn ico_mic(size: f32, color: u32, bg: Option<u32>) -> Div {
    icon_container("icons/mic.svg", size, color, bg)
}

pub fn ico_speaker(size: f32, color: u32, bg: Option<u32>) -> Div {
    icon_container("icons/speaker.svg", size, color, bg)
}

pub fn app_icon(icon: crate::models::AppIcon, size: f32, color: u32, bg: Option<u32>) -> Div {
    match icon {
        crate::models::AppIcon::Music => ico_music(size, color, bg),
        crate::models::AppIcon::Mic => ico_mic(size, color, bg),
        crate::models::AppIcon::Speaker => ico_speaker(size, color, bg),
    }
}

// ── Output device icons (emerald/teal theme) ─────────────────────────────────
pub fn ico_headphones(size: f32, color: u32, bg: Option<u32>) -> Div {
    icon_container("icons/headphones.svg", size, color, bg)
}

pub fn ico_monitor(size: f32, color: u32, bg: Option<u32>) -> Div {
    icon_container("icons/monitor.svg", size, color, bg)
}

pub fn output_device_icon(device_type: &str, size: f32, color: u32, bg: Option<u32>) -> Div {
    match device_type {
        "headphones" => icon_container("icons/headphones.svg", size, color, bg),
        "monitor" => icon_container("icons/monitor.svg", size, color, bg),
        _ => icon_container("icons/speaker.svg", size, color, bg),
    }
}

// ── Input device icon (amber/orange theme) ───────────────────────────────────
pub fn ico_mic2(size: f32, color: u32, bg: Option<u32>) -> Div {
    icon_container("icons/mic2.svg", size, color, bg)
}

// ── Volume / UI icons ────────────────────────────────────────────────────────
pub fn ico_volume_on(size: f32, color: u32) -> Div {
    div().child(svg().path("icons/volume2.svg").size(px(size)).text_color(rgb(color)))
}

pub fn ico_volume_off(size: f32, color: u32) -> Div {
    div().child(svg().path("icons/volumex.svg").size(px(size)).text_color(rgb(color)))
}

pub fn ico_plus(size: f32, color: u32) -> Div {
    div().child(svg().path("icons/plus.svg").size(px(size)).text_color(rgb(color)))
}

pub fn ico_x(size: f32, color: u32) -> Div {
    div().child(svg().path("icons/x.svg").size(px(size)).text_color(rgb(color)))
}

pub fn ico_chevron_down(size: f32, color: u32) -> Div {
    div().child(svg().path("icons/chevrondown.svg").size(px(size)).text_color(rgb(color)))
}
