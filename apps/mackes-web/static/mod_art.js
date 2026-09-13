(function () {
  const root = '/assets/vendor/mod-art/';
  const assets = Object.freeze({
    rack: 'surfaces/rack.png', library: 'surfaces/library-background.png',
    knob: 'controls/knob.png', slider: 'controls/slider.png', footswitch: 'controls/footswitch.png',
    switch: 'controls/switch.svg', redOn: 'status/red-on.png', redOff: 'status/red-off.png',
    purpleOn: 'status/purple-on.png', purpleOff: 'status/purple-off.png',
    audioIn: 'ports/audio-input.png', audioOut: 'ports/audio-output.png',
    audioJack: 'ports/audio-jack.png', audioOpen: 'ports/audio-jack-open.png',
    midiIn: 'ports/midi-input.png', midiOut: 'ports/midi-output.png',
    midiJack: 'ports/midi-jack.png', midiOpen: 'ports/midi-jack-open.png',
    cvIn: 'ports/cv-input.png', cvOut: 'ports/cv-output.png',
    cvJack: 'ports/cv-jack.png', cvOpen: 'ports/cv-jack-open.png',
    blocked: 'icons/blocked.svg', brokenImage: 'icons/broken-image.svg',
    brokenPedal: 'icons/broken-pedal.svg', transport: 'icons/transport.svg',
  });
  const stateLabels = Object.freeze({ observed: 'Observed', acknowledged: 'Acknowledged', pending: 'Pending', stale: 'Stale', unavailable: 'Unavailable', disconnected: 'Disconnected', conflict: 'Conflict', 'sent-unverified': 'Sent, unverified' });
  function url(key) { return assets[key] ? root + assets[key] : ''; }
  function stateLabel(state) { return stateLabels[String(state || '').toLowerCase()] || 'Unknown'; }
  function image(key, options = {}) {
    const source = url(key);
    if (!source) return null;
    const element = document.createElement('img');
    element.src = source;
    element.alt = options.alt || '';
    element.className = `mod-art mod-art-${key}${options.state ? ` is-${options.state}` : ''}`;
    element.dataset.assetId = `mod-${key}`;
    if (!options.alt) element.setAttribute('aria-hidden', 'true');
    if (options.width) element.width = options.width;
    if (options.height) element.height = options.height;
    return element;
  }
  function appendState(parent, state, detail = '') {
    const text = document.createElement('span');
    text.className = `mod-art-state is-${String(state || 'unknown').toLowerCase()}`;
    text.textContent = `${stateLabel(state)}${detail ? ` · ${detail}` : ''}`;
    text.setAttribute('role', 'status');
    parent.append(text);
    return text;
  }
  function semanticControl(options = {}) {
    const element = document.createElement(options.tag || 'button');
    element.type = options.tag === 'button' || !options.tag ? 'button' : undefined;
    element.className = `mod-art-control ${options.className || ''}`.trim();
    element.dataset.capability = options.capability || 'unavailable';
    element.dataset.state = options.state || 'unavailable';
    element.setAttribute('aria-label', options.label || 'Unavailable control');
    if (options.disabled || options.capability === 'unavailable') element.disabled = true;
    if (options.image) {
      const art = image(options.image, { alt: '' });
      if (art) element.append(art);
    }
    const label = document.createElement('span');
    label.className = 'mod-art-control-label';
    label.textContent = options.label || 'Unavailable control';
    element.append(label);
    appendState(element, options.state, options.detail);
    return element;
  }
  function pluginFace(options = {}) {
    const face = document.createElement('span');
    face.className = 'mod-art-plugin-face';
    face.dataset.renderer = options.renderer || 'generic';
    const art = image(faceAssetKey(options.renderer), { alt: '' });
    if (art) face.append(art);
    const name = document.createElement('span'); name.className = 'mod-art-plugin-name'; name.textContent = options.name || 'Generic plugin'; face.append(name);
    const state = document.createElement('span'); state.className = 'mod-art-plugin-state'; state.textContent = stateLabel(options.state || 'unavailable'); face.append(state);
    return face;
  }
  function faceAssetKey(renderer) { return /pipedal|rack/i.test(String(renderer || '')) ? 'rack' : 'library'; }
  window.MackesModArt = Object.freeze({ assets, url, stateLabel, image, appendState, semanticControl, pluginFace, faceAssetKey });
}());
