(function () {
  const host = document.getElementById('mod-art-gallery');
  if (!host || !window.MackesModArt) return;
  const samples = [
    ['knob', 'Writable rotary', 'observed', 'default'], ['slider', 'Writable fader', 'pending', 'hover'],
    ['footswitch', 'Footswitch', 'acknowledged', 'focus'], ['redOn', 'Confirmed status LED', 'observed', 'active'],
    ['purpleOff', 'Unavailable status LED', 'unavailable', 'disabled'], ['audioJack', 'Audio connection', 'acknowledged', 'default'],
    ['midiOpen', 'MIDI open port', 'disconnected', 'stale'], ['cvJack', 'CV connection', 'stale', 'pending'],
    ['brokenImage', 'Missing-art fallback', 'unavailable', 'fallback'], ['blocked', 'Blocked operation', 'conflict', 'blocked'],
    ['switch', 'Sent without readback', 'sent-unverified', 'pending'],
  ];
  for (const [asset, label, state, interaction] of samples) {
    const item = document.createElement('article');
    item.className = `mod-art-gallery-item is-${interaction}`;
    item.setAttribute('aria-label', `${label}, ${window.MackesModArt.stateLabel(state)}`);
    const art = window.MackesModArt.image(asset, { alt: label, width: 88, height: 56, state });
    if (art) item.append(art);
    const heading = document.createElement('strong'); heading.textContent = label; item.append(heading);
    window.MackesModArt.appendState(item, state, 'semantic state example');
    host.append(item);
  }
  const controls = [
    ['knob', 'Keyboard rotary', 'writable', 'observed'],
    ['footswitch', 'Unavailable footswitch', 'unavailable', 'unavailable'],
  ];
  for (const [image, label, capability, state] of controls) {
    const item = document.createElement('article');
    item.className = 'mod-art-gallery-item';
    item.append(window.MackesModArt.semanticControl({ image, label, capability, state, detail: capability === 'writable' ? 'Enter to activate' : 'Disabled safely' }));
    host.append(item);
  }
}());
