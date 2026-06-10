# KernelPilot

A PipeWire audio mixer with a native desktop GUI built in Rust using [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui).

## Features

- **Per-app volume control** — independently adjust volume and mute for each playing application
- **Output routing** — route any app to any output device (speakers, headphones, HDMI) with per-port switching
- **Live PipeWire graph** — monitors nodes, ports, and links in real time via the PipeWire registry
- **Clean restore on exit** — disconnects custom links and resets all stream volumes/mutes back to defaults
- **Custom window chrome** — client-side decorations with drag-to-move titlebar and resize handles

## Requirements

- Linux with PipeWire running
- `pactl` (PulseAudio utilities, works with PipeWire's PulseAudio layer)
- `pw-link` (from `pipewire-utils` or similar)
- Rust toolchain (stable)

## Build & Run

```sh
cargo run --release
```

## Architecture

```
src/
├── main.rs                  # App entry, window setup
├── models.rs                # Data types: AudioNode, MixerState, AppGroup, EngineEvent/Command
├── state.rs                 # GPUI global (shared state + channel ends)
├── audio/
│   └── pipewire_engine.rs   # PipeWire monitor thread + pactl/pw-link command thread
└── ui/
    ├── mixer.rs             # Main window render tree, event polling, drag sliders
    └── icons.rs             # SVG icon helpers
```

### Data Flow

```
PipeWire registry
    │ (pipewire crate callbacks)
    ▼
monitor thread ──EngineEvent──► poll_events() in render loop
                                      │
                                 MixerState mutation → re-render

User interaction (click / drag)
    │
    ▼
EngineCommand ──channel──► command thread ──► pactl / pw-link
```

### Key Types

| Type | Purpose |
|---|---|
| `AudioNode` | A PipeWire node (app stream, sink, source, virtual bus) |
| `NodeType` | `App`, `AppInput`, `OutputDevice`, `InputDevice`, `VirtualBus` |
| `AppGroup` | Merges playback + capture streams of the same app into one card |
| `OutputTarget` | A routing destination — whole sink or a specific port (e.g. Speakers vs Headphones) |
| `EngineEvent` | Node/port/link add/remove, volume/mute changes sent from audio thread to UI |
| `EngineCommand` | SetVolume, SetMute, CreateLink, RemoveLink, SetSinkPort sent from UI to audio thread |

## How Routing Works

1. The UI sends `EngineCommand::CreateLink { from_name, to_name }` when the user picks an output.
2. The command thread calls `pw-link` to enumerate real port names for both nodes, then pairs them by channel suffix (FL↔FL, FR↔FR).
3. To switch between ports on the same sink (e.g. Speakers → Headphones), a `SetSinkPort` command runs `pactl set-sink-port` before creating the new link.
4. When a new output device appears (e.g. plugging in headphones), existing app→device links are automatically restored via `EngineCommand::RestoreLinks`.

## License

MIT
