import { useState } from 'react';
import { Volume2, VolumeX, Music, Mic, Headphones, Speaker, MonitorSpeaker, Mic2, Plus, X, ChevronDown } from 'lucide-react';
import * as Slider from '@radix-ui/react-slider';
import * as Select from '@radix-ui/react-select';
import * as Popover from '@radix-ui/react-popover';

interface AudioStream {
  id: string;
  name: string;
  icon: 'music' | 'mic' | 'speaker';
  volume: number;
  muted: boolean;
  outputs: string[];
  input: string | null;
}

interface Device {
  id: string;
  name: string;
  type: 'headphones' | 'speakers' | 'monitor';
  volume: number;
  muted: boolean;
}

interface InputDevice {
  id: string;
  name: string;
  type: 'mic';
  volume: number;
  muted: boolean;
}

export default function App() {
  const [outputDevices, setOutputDevices] = useState<Device[]>([
    { id: 'out-1', name: 'Built-in Audio Analog Stereo', type: 'speakers', volume: 80, muted: false },
    { id: 'out-2', name: 'USB Headphones', type: 'headphones', volume: 65, muted: false },
    { id: 'out-3', name: 'HDMI Monitor Audio', type: 'monitor', volume: 90, muted: false },
  ]);

  const [inputDevices, setInputDevices] = useState<InputDevice[]>([
    { id: 'in-1', name: 'Built-in Microphone', type: 'mic', volume: 70, muted: false },
    { id: 'in-2', name: 'USB Microphone', type: 'mic', volume: 85, muted: false },
  ]);

  const [streams, setStreams] = useState<AudioStream[]>([
    { id: '1', name: 'Firefox', icon: 'music', volume: 75, muted: false, outputs: ['out-1'], input: null },
    { id: '2', name: 'Spotify', icon: 'music', volume: 60, muted: false, outputs: ['out-2'], input: null },
    { id: '3', name: 'Discord', icon: 'mic', volume: 85, muted: false, outputs: ['out-2', 'out-3'], input: 'in-2' },
    { id: '4', name: 'OBS Studio', icon: 'speaker', volume: 90, muted: false, outputs: ['out-1', 'out-3'], input: 'in-1' },
    { id: '5', name: 'VLC Media Player', icon: 'music', volume: 50, muted: true, outputs: ['out-1'], input: null },
  ]);

  const updateStreamVolume = (id: string, volume: number) => {
    setStreams(streams.map(s => s.id === id ? { ...s, volume } : s));
  };

  const toggleStreamMute = (id: string) => {
    setStreams(streams.map(s => s.id === id ? { ...s, muted: !s.muted } : s));
  };

  const updateOutputDeviceVolume = (id: string, volume: number) => {
    setOutputDevices(outputDevices.map(d => d.id === id ? { ...d, volume } : d));
  };

  const toggleOutputDeviceMute = (id: string) => {
    setOutputDevices(outputDevices.map(d => d.id === id ? { ...d, muted: !d.muted } : d));
  };

  const updateInputDeviceVolume = (id: string, volume: number) => {
    setInputDevices(inputDevices.map(d => d.id === id ? { ...d, volume } : d));
  };

  const toggleInputDeviceMute = (id: string) => {
    setInputDevices(inputDevices.map(d => d.id === id ? { ...d, muted: !d.muted } : d));
  };

  const addOutputToStream = (streamId: string, outputId: string) => {
    setStreams(streams.map(s =>
      s.id === streamId && !s.outputs.includes(outputId)
        ? { ...s, outputs: [...s.outputs, outputId] }
        : s
    ));
  };

  const removeOutputFromStream = (streamId: string, outputId: string) => {
    setStreams(streams.map(s =>
      s.id === streamId
        ? { ...s, outputs: s.outputs.filter(id => id !== outputId) }
        : s
    ));
  };

  const updateStreamInput = (streamId: string, inputId: string | null) => {
    setStreams(streams.map(s =>
      s.id === streamId ? { ...s, input: inputId } : s
    ));
  };

  const getIcon = (type: string) => {
    switch (type) {
      case 'music': return <Music className="w-5 h-5" />;
      case 'mic': return <Mic className="w-5 h-5" />;
      case 'speaker': return <Speaker className="w-5 h-5" />;
      default: return <Music className="w-5 h-5" />;
    }
  };

  const getOutputIcon = (type: string) => {
    switch (type) {
      case 'headphones': return <Headphones className="w-4 h-4" />;
      case 'speakers': return <Speaker className="w-4 h-4" />;
      case 'monitor': return <MonitorSpeaker className="w-4 h-4" />;
      default: return <Speaker className="w-4 h-4" />;
    }
  };

  const VolumeSlider = ({ volume, muted, onVolumeChange, onToggleMute }: {
    volume: number;
    muted: boolean;
    onVolumeChange: (v: number) => void;
    onToggleMute: () => void;
  }) => (
    <div className="flex items-center gap-3 flex-1">
      <button
        onClick={onToggleMute}
        className={`p-2 rounded-full transition-all ${muted ? 'bg-rose-500/20 text-rose-400' : 'hover:bg-white/5 text-gray-400'}`}
      >
        {muted ? (
          <VolumeX className="w-4 h-4" />
        ) : (
          <Volume2 className="w-4 h-4" />
        )}
      </button>

      <Slider.Root
        className="relative flex items-center select-none touch-none flex-1 h-5"
        value={[volume]}
        max={100}
        step={1}
        onValueChange={([value]) => onVolumeChange(value)}
        disabled={muted}
      >
        <Slider.Track className="bg-white/5 relative grow h-2 rounded-full overflow-hidden">
          <Slider.Range className="absolute bg-gradient-to-r from-violet-500 via-purple-500 to-fuchsia-500 h-full rounded-full" />
        </Slider.Track>
        <Slider.Thumb
          className="block w-4 h-4 bg-white rounded-full hover:scale-110 focus:outline-none disabled:opacity-30 transition-all shadow-lg shadow-purple-500/30"
          aria-label="Volume"
        />
      </Slider.Root>

      <div className="w-11 text-right text-sm tabular-nums text-gray-400 font-medium">
        {muted ? '0%' : `${volume}%`}
      </div>
    </div>
  );

  return (
    <div className="dark min-h-screen bg-gradient-to-br from-[#0a0e1a] via-[#0d1117] to-[#0a0e1a] text-foreground p-6" style={{ fontFamily: "'DM Sans', sans-serif" }}>
      <div className="max-w-5xl mx-auto">
        {/* Header */}
        <div className="mb-6">
          <h1 className="text-2xl mb-1 text-transparent bg-clip-text bg-gradient-to-r from-violet-400 via-purple-400 to-fuchsia-400 font-semibold">
            PipeWire Mixer
          </h1>
          <p className="text-sm text-gray-500">
            {streams.length} applications • {outputDevices.length} outputs • {inputDevices.length} inputs
          </p>
        </div>

        {/* Applications */}
        <div className="mb-5">
          <h2 className="text-sm font-semibold text-gray-400 mb-3 px-1">Applications</h2>
          <div className="space-y-3">
            {streams.map(stream => {
              const inputDevice = stream.input ? inputDevices.find(d => d.id === stream.input) : null;

              return (
                <div
                  key={stream.id}
                  className="bg-white/[0.02] backdrop-blur-xl rounded-2xl p-4 hover:bg-white/[0.04] transition-all"
                >
                  <div className="flex items-center gap-4 mb-3">
                    <div className="flex items-center gap-3 w-48">
                      <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-violet-500/20 to-fuchsia-500/20 flex items-center justify-center text-violet-400 shadow-lg shadow-violet-500/10">
                        {getIcon(stream.icon)}
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium text-gray-200 truncate">{stream.name}</div>
                        <div className="text-xs text-gray-500">
                          {stream.outputs.length} out • {stream.input ? '1' : '0'} in
                        </div>
                      </div>
                    </div>

                    <VolumeSlider
                      volume={stream.volume}
                      muted={stream.muted}
                      onVolumeChange={(v) => updateStreamVolume(stream.id, v)}
                      onToggleMute={() => toggleStreamMute(stream.id)}
                    />
                  </div>

                  {/* Routing */}
                  <div className="flex items-start gap-4 text-xs">
                    <div className="flex-1">
                      <div className="text-xs text-gray-500 mb-2 font-medium">Output</div>
                      <div className="flex items-center gap-2 flex-wrap">
                        {stream.outputs.map(outputId => {
                          const device = outputDevices.find(d => d.id === outputId);
                          if (!device) return null;

                          return (
                            <div
                              key={outputId}
                              className="inline-flex items-center gap-2 bg-emerald-500/10 rounded-full px-3 py-1.5 text-emerald-400 text-xs font-medium"
                            >
                              {getOutputIcon(device.type)}
                              <span>{device.name}</span>
                              <button
                                onClick={() => removeOutputFromStream(stream.id, outputId)}
                                className="hover:bg-emerald-500/20 rounded-full transition-colors p-0.5"
                              >
                                <X className="w-3.5 h-3.5" />
                              </button>
                            </div>
                          );
                        })}

                        <Popover.Root>
                          <Popover.Trigger className="inline-flex items-center gap-1.5 bg-white/5 hover:bg-white/10 rounded-full px-3 py-1.5 text-gray-400 transition-all text-xs font-medium">
                            <Plus className="w-3.5 h-3.5" />
                            Add
                          </Popover.Trigger>

                          <Popover.Portal>
                            <Popover.Content
                              className="bg-[#161b22] backdrop-blur-xl border border-white/10 rounded-2xl shadow-2xl p-2 min-w-[220px]"
                              sideOffset={5}
                            >
                              {outputDevices
                                .filter(device => !stream.outputs.includes(device.id))
                                .map(device => (
                                  <button
                                    key={device.id}
                                    onClick={() => addOutputToStream(stream.id, device.id)}
                                    className="w-full flex items-center gap-2 px-3 py-2 rounded-xl hover:bg-white/5 text-left text-gray-300 text-sm"
                                  >
                                    {getOutputIcon(device.type)}
                                    <span>{device.name}</span>
                                  </button>
                                ))}
                              {outputDevices.filter(device => !stream.outputs.includes(device.id)).length === 0 && (
                                <div className="px-3 py-2 text-sm text-gray-500">
                                  All outputs assigned
                                </div>
                              )}
                            </Popover.Content>
                          </Popover.Portal>
                        </Popover.Root>
                      </div>
                    </div>

                    <div className="flex-1">
                      <div className="text-xs text-gray-500 mb-2 font-medium">Input</div>
                      <Select.Root value={stream.input || 'none'} onValueChange={(value) => updateStreamInput(stream.id, value === 'none' ? null : value)}>
                        <Select.Trigger className="inline-flex items-center justify-between gap-2 rounded-full px-3 py-1.5 bg-amber-500/10 hover:bg-amber-500/15 transition-all w-full max-w-[220px] text-amber-400 text-xs font-medium">
                          <div className="flex items-center gap-2 min-w-0">
                            <Mic2 className="w-3.5 h-3.5 flex-shrink-0" />
                            <span className="truncate">
                              {inputDevice ? inputDevice.name : 'None'}
                            </span>
                          </div>
                          <Select.Icon>
                            <ChevronDown className="w-3.5 h-3.5 flex-shrink-0" />
                          </Select.Icon>
                        </Select.Trigger>

                        <Select.Portal>
                          <Select.Content className="overflow-hidden bg-[#161b22] backdrop-blur-xl border border-white/10 rounded-2xl shadow-2xl">
                            <Select.Viewport className="p-2">
                              <Select.Item
                                value="none"
                                className="relative flex items-center gap-2 pl-8 pr-3 py-2 rounded-xl cursor-pointer outline-none hover:bg-white/5 data-[highlighted]:bg-white/5 text-gray-400 text-sm"
                              >
                                <Select.ItemIndicator className="absolute left-2 inline-flex items-center">
                                  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                                    <path d="M10 3L4.5 8.5L2 6" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
                                  </svg>
                                </Select.ItemIndicator>
                                <Select.ItemText>None</Select.ItemText>
                              </Select.Item>
                              {inputDevices.map(device => (
                                <Select.Item
                                  key={device.id}
                                  value={device.id}
                                  className="relative flex items-center gap-2 pl-8 pr-3 py-2 rounded-xl cursor-pointer outline-none hover:bg-white/5 data-[highlighted]:bg-white/5 text-gray-300 text-sm"
                                >
                                  <Select.ItemIndicator className="absolute left-2 inline-flex items-center">
                                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                                      <path d="M10 3L4.5 8.5L2 6" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
                                    </svg>
                                  </Select.ItemIndicator>
                                  <Mic2 className="w-3.5 h-3.5" />
                                  <Select.ItemText>{device.name}</Select.ItemText>
                                </Select.Item>
                              ))}
                            </Select.Viewport>
                          </Select.Content>
                        </Select.Portal>
                      </Select.Root>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Output Devices */}
        <div className="mb-5">
          <h2 className="text-sm font-semibold text-gray-400 mb-3 px-1">Output Devices</h2>
          <div className="space-y-2">
            {outputDevices.map(device => (
              <div
                key={device.id}
                className="bg-white/[0.02] backdrop-blur-xl rounded-2xl p-4 hover:bg-white/[0.04] transition-all"
              >
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-3 w-48">
                    <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-emerald-500/20 to-teal-500/20 flex items-center justify-center text-emerald-400 shadow-lg shadow-emerald-500/10">
                      {getOutputIcon(device.type)}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium text-gray-200 truncate">{device.name}</div>
                      <div className="text-xs text-gray-500">Output</div>
                    </div>
                  </div>

                  <VolumeSlider
                    volume={device.volume}
                    muted={device.muted}
                    onVolumeChange={(v) => updateOutputDeviceVolume(device.id, v)}
                    onToggleMute={() => toggleOutputDeviceMute(device.id)}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Input Devices */}
        <div>
          <h2 className="text-sm font-semibold text-gray-400 mb-3 px-1">Input Devices</h2>
          <div className="space-y-2">
            {inputDevices.map(device => (
              <div
                key={device.id}
                className="bg-white/[0.02] backdrop-blur-xl rounded-2xl p-4 hover:bg-white/[0.04] transition-all"
              >
                <div className="flex items-center gap-4">
                  <div className="flex items-center gap-3 w-48">
                    <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-amber-500/20 to-orange-500/20 flex items-center justify-center text-amber-400 shadow-lg shadow-amber-500/10">
                      <Mic2 className="w-5 h-5" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium text-gray-200 truncate">{device.name}</div>
                      <div className="text-xs text-gray-500">Input</div>
                    </div>
                  </div>

                  <VolumeSlider
                    volume={device.volume}
                    muted={device.muted}
                    onVolumeChange={(v) => updateInputDeviceVolume(device.id, v)}
                    onToggleMute={() => toggleInputDeviceMute(device.id)}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
