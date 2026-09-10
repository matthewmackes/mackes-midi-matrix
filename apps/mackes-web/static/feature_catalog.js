(function () {
  const catalog = [
    { match: /novation|launch control/i, name: 'Novation Launch Control XL', profile: 'novation.launch-control-xl', source: 'Qualified profile + Programmer Reference', features: ['24 knobs', '16 channel buttons', '8 utility controls', '8 faders', 'LED intent / delivery state', 'Template and reconnect diagnostics'], operations: [{ label: 'Resync device (confirmed)', operation: 'rescan' }] },
    { match: /eventide|micropitch/i, name: 'Eventide MicroPitch Delay', profile: 'eventide.micropitch', source: 'MicroPitch QRG + Eventide profile', features: [{ label: 'Expression', control: 'expression', cc: 4 }, { label: 'Tap trigger', control: 'tap', cc: 9 }, { label: 'Active / bypass', control: 'active', cc: 14 }, { label: 'FLEX', control: 'flex', cc: 15 }, { label: 'Mix', control: 'mix', cc: 20 }, { label: 'Pitch A', control: 'pitch-a', cc: 21 }, { label: 'Pitch B', control: 'pitch-b', cc: 22 }, { label: 'Depth', control: 'depth', cc: 23 }, { label: 'Rate sensitivity', control: 'rate-sensitivity', cc: 24 }, { label: 'Pitch mix', control: 'pitch-mix', cc: 25 }, { label: 'Tone', control: 'tone', cc: 26 }, { label: 'Delay A', control: 'delay-a', cc: 27 }, { label: 'Delay B', control: 'delay-b', cc: 28 }, { label: 'Modulation', control: 'modulation', cc: 29 }, { label: 'Feedback', control: 'feedback', cc: 30 }, { label: 'Output level', control: 'output-level', cc: 31 }] },
    { match: /lexicon|reflex/i, name: 'Lexicon Reflex', profile: 'lexicon.reflex', source: 'Reflex MIDI implementation + codec metadata', features: [{ label: 'Algorithm selector', control: 'algorithm-select' }, { label: 'Algorithm parameters', control: 'parameter' }, { label: 'Echo Rhythm', control: 'echo-rhythm' }, { label: 'MIDI patch 1–4', control: 'midi-patch' }, { label: 'Register read / recall', control: 'register-recall' }, { label: 'Register store (persistent)', control: 'register-store' }, { label: 'Setup / dump diagnostics', control: 'setup-dump' }, { label: 'System reset (confirmed)', control: 'system-reset' }, { label: 'Bypass / system task', control: 'bypass-task' }] },
    { match: /pipedal/i, name: 'PiPedal', profile: 'pipedal', source: 'Pinned server operation audit', features: ['Dynamic plugin controls', 'Snapshots and presets', 'MIDI bindings', 'Levels and pedalboard state', 'Unsupported operation reasons'] },
    { match: /midisport|m-audio/i, name: 'M-Audio MIDISPORT 4x4', profile: 'm-audio.midisport-4x4', source: 'Manufacturer capability evidence', features: ['4 MIDI inputs', '4 MIDI outputs', 'Per-port activity', 'Direction-aware aliases', 'Firmware and cable state separation'] },
    { match: /rtp|apple midi/i, name: 'RTP-MIDI session', profile: 'rtp-midi', source: 'AppleMIDI/RFC session requirements', features: ['Peer identity and allowlist', 'Invitation/handshake lifecycle', 'Sequence and reorder health', 'Reconnect backoff', 'Message delivery counters'] },
    { match: /midi through|generic midi/i, name: 'Generic MIDI transport', profile: 'generic-midi', source: 'Qualified MIDI engine and endpoint schemas', features: ['Direction-aware ports', 'Channel and message filters', 'CC/note/program/pressure/bend', 'SysEx and realtime predicates', 'Stable identity and reconnect repair'] }
  ];
  function entriesFor(devices) {
    const names = devices.map(device => String(device.name || device.alias || device.id || ''));
    return catalog.map(item => ({
      ...item,
      connected: names.some(name => item.match.test(name))
    }));
  }
  window.MackesFeatureCatalog = Object.freeze({ catalog, entriesFor });
}());
