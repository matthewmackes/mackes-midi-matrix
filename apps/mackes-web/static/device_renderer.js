(function () {
  const namespace = ['http', String.fromCharCode(58, 47, 47), 'www.w3.org/2000/svg'].join('');
  const renderers = [
    { key: 'novation.launch-control-xl', test: /novation|launch control/i, kind: 'controller', accent: '#4589ff', controls: 24 },
    { key: 'eventide.micropitch', test: /eventide|micropitch/i, kind: 'pedal', accent: '#ee5396', controls: 6 },
    { key: 'lexicon.reflex', test: /lexicon|reflex/i, kind: 'rack', accent: '#be95ff', controls: 8 },
    { key: 'pipedal', test: /pipedal/i, kind: 'pedalboard', accent: '#42be65', controls: 5 },
    { key: 'm-audio.midisport-4x4', test: /midisport|m-audio/i, kind: 'interface', accent: '#08bdba', controls: 8 },
    { key: 'rtp-midi', test: /rtp|apple midi/i, kind: 'network', accent: '#f1c21b', controls: 4 },
    { key: 'generic-midi', test: /midi through|generic midi/i, kind: 'midi', accent: '#8d8d8d', controls: 2 },
    { key: 'mackes.virtual-monitor', test: /mackes|virtual|monitor/i, kind: 'virtual', accent: '#78a9ff', controls: 2 },
  ];
  const fallback = { key: 'generic.endpoint', kind: 'unknown', accent: '#a8a8a8', controls: 2 };
  function rendererFor(device) {
    const name = String(device?.name || device?.alias || device?.kind || '');
    return renderers.find(renderer => renderer.test.test(name)) || fallback;
  }
  function stateModel(device) {
    const state = String(device?.state || device?.connection_state || 'unknown').toLowerCase();
    const label = state === 'ready' || state === 'connected' ? 'Connected' : state === 'degraded' ? 'Needs attention' : state === 'disconnected' ? 'Disconnected' : 'Unknown';
    const input = device?.input_count ?? device?.inputs ?? device?.input;
    const output = device?.output_count ?? device?.outputs ?? device?.output;
    const direction = input || output ? `${input ? 'MIDI in' : ''}${input && output ? ' · ' : ''}${output ? 'MIDI out' : ''}` : 'MIDI direction unavailable';
    return { state, label, direction };
  }
  function svgElement(tag, attrs, parent) {
    const element = document.createElementNS(namespace, tag);
    Object.entries(attrs).forEach(([key, value]) => element.setAttribute(key, value));
    parent.append(element);
    return element;
  }
  function appendGraphic(device, parent) {
    const renderer = rendererFor(device);
    const svg = svgElement('svg', { class: 'device-graphic', viewBox: '0 0 360 150', role: 'img', 'aria-label': `${renderer.key} graphical device representation` }, parent);
    svgElement('rect', { x: 4, y: 4, width: 352, height: 142, rx: 10, class: 'device-graphic-chassis', 'data-renderer-key': renderer.key }, svg);
    svgElement('rect', { x: 4, y: 4, width: 8, height: 142, rx: 4, fill: renderer.accent, 'aria-hidden': 'true' }, svg);
    const title = svgElement('text', { x: 24, y: 30, class: 'device-graphic-title' }, svg); title.textContent = renderer.kind;
    const model = stateModel(device);
    const state = model.label;
    const status = svgElement('text', { x: 24, y: 51, class: 'device-graphic-state' }, svg); status.textContent = state;
    if (renderer.kind === 'controller') {
      for (let index = 0; index < 8; index += 1) svgElement('line', { x1: 35 + index * 38, y1: 121, x2: 35 + index * 38, y2: 83, class: 'device-graphic-fader' }, svg);
      for (let index = 0; index < renderer.controls; index += 1) svgElement('circle', { cx: 35 + (index % 8) * 38, cy: 69 + Math.floor(index / 8) * 18, r: 5, class: 'device-graphic-control' }, svg);
    } else if (renderer.key === 'eventide.micropitch') {
      svgElement('rect', { x: 38, y: 61, width: 284, height: 66, rx: 6, class: 'device-graphic-panel' }, svg);
      svgElement('rect', { x: 50, y: 70, width: 74, height: 17, rx: 2, class: 'device-graphic-display' }, svg);
      const eventideLabel = svgElement('text', { x: 57, y: 82, class: 'device-graphic-caption' }, svg); eventideLabel.textContent = 'MICROPITCH';
      for (let index = 0; index < 4; index += 1) svgElement('circle', { cx: 151 + index * 35, cy: 79, r: 6, class: 'device-graphic-control' }, svg);
      svgElement('circle', { cx: 180, cy: 111, r: 10, class: 'device-graphic-switch' }, svg);
      svgElement('circle', { cx: 276, cy: 111, r: 10, class: 'device-graphic-switch' }, svg);
    } else if (renderer.key === 'lexicon.reflex') {
      svgElement('rect', { x: 28, y: 59, width: 304, height: 68, rx: 4, class: 'device-graphic-panel' }, svg);
      svgElement('rect', { x: 45, y: 71, width: 86, height: 22, rx: 2, class: 'device-graphic-display' }, svg);
      for (let index = 0; index < 4; index += 1) svgElement('rect', { x: 148 + index * 38, y: 72, width: 22, height: 10, rx: 2, class: 'device-graphic-control' }, svg);
      for (let index = 0; index < 5; index += 1) svgElement('line', { x1: 52 + index * 42, y1: 111, x2: 52 + index * 42, y2: 99 + (index % 2) * 5, class: 'device-graphic-meter' }, svg);
      const reflexLabel = svgElement('text', { x: 148, y: 112, class: 'device-graphic-caption' }, svg); reflexLabel.textContent = 'ALGORITHM · ECHO RHYTHM · MIDI';
    } else if (renderer.key === 'pipedal') {
      svgElement('rect', { x: 28, y: 60, width: 304, height: 67, rx: 7, class: 'device-graphic-panel' }, svg);
      for (let index = 0; index < 3; index += 1) {
        const x = 42 + index * 94;
        svgElement('rect', { x, y: 77, width: 72, height: 27, rx: 4, class: 'device-graphic-plugin' }, svg);
        if (index < 2) svgElement('path', { d: `M ${x + 74} 90 L ${x + 89} 90`, class: 'device-graphic-link' }, svg);
      }
      const pedalLabel = svgElement('text', { x: 43, y: 119, class: 'device-graphic-caption' }, svg); pedalLabel.textContent = 'PEDALBOARD · PLUGINS · LEVELS';
    } else if (renderer.kind === 'pedal') {
      svgElement('rect', { x: 30, y: 62, width: 300, height: 62, rx: 8, class: 'device-graphic-panel' }, svg);
      for (let index = 0; index < renderer.controls; index += 1) svgElement('circle', { cx: 58 + index * (250 / Math.max(1, renderer.controls - 1)), cy: 83, r: 7, class: 'device-graphic-control' }, svg);
      svgElement('circle', { cx: 180, cy: 112, r: 9, class: 'device-graphic-switch' }, svg);
    } else if (renderer.kind === 'interface') {
      for (let index = 0; index < 4; index += 1) {
        svgElement('circle', { cx: 58 + index * 52, cy: 84, r: 8, class: 'device-graphic-port-in' }, svg);
        svgElement('circle', { cx: 58 + index * 52, cy: 112, r: 8, class: 'device-graphic-port-out' }, svg);
      }
    } else if (renderer.kind === 'network') {
      svgElement('path', { d: 'M 60 105 C 110 60, 150 130, 200 85 S 285 70, 310 105', class: 'device-graphic-network' }, svg);
      for (let index = 0; index < renderer.controls; index += 1) svgElement('circle', { cx: 60 + index * 83, cy: 105 - (index % 2) * 20, r: 8, class: 'device-graphic-port-out' }, svg);
    } else {
      for (let index = 0; index < renderer.controls; index += 1) svgElement('rect', { x: 55 + index * 120, y: 82, width: 52, height: 28, rx: 5, class: 'device-graphic-port' }, svg);
    }
    const ports = svgElement('text', { x: 24, y: 139, class: 'device-graphic-caption' }, svg); ports.textContent = renderer.kind === 'interface' ? '4 inputs · 4 outputs' : model.direction;
    const accessibleList = document.createElement('ul');
    accessibleList.className = 'device-graphic-accessible-list';
    accessibleList.setAttribute('aria-label', `${renderer.key} controls and connection summary`);
    const summary = renderer.kind === 'controller'
      ? ['24 knobs', '16 channel buttons', '8 utility controls', '8 faders', 'LED and assignment state']
      : renderer.kind === 'interface'
        ? ['4 MIDI inputs', '4 MIDI outputs', 'Cable and route state']
        : renderer.key === 'pipedal'
          ? ['Pedalboard plugin graph', 'Plugin controls and levels', 'Preset and snapshot state']
          : renderer.key === 'eventide.micropitch'
            ? ['Parameter controls', 'Two footswitches', 'Preset and sent-unverified state']
            : renderer.key === 'lexicon.reflex'
              ? ['Rack parameters', 'Algorithm and Echo Rhythm', 'Patch and register state']
              : renderer.kind === 'network'
                ? ['Peer and session state', 'Handshake and reconnect state', 'Virtual MIDI ports']
                : ['Named MIDI ports', 'Availability state', 'Activity and capability summary'];
    for (const item of [`State: ${state}`, ...summary]) {
      const listItem = document.createElement('li');
      listItem.textContent = item;
      accessibleList.append(listItem);
    }
    parent.append(accessibleList);
    return renderer;
  }
  window.MackesDeviceRenderer = Object.freeze({
    keys: Object.freeze([...renderers.map(renderer => renderer.key), fallback.key]),
    rendererFor,
    stateModel,
    appendGraphic,
  });
}());
