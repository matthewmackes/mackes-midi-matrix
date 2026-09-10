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
    const state = String(device?.state || device?.connection_state || 'unknown');
    const status = svgElement('text', { x: 24, y: 51, class: 'device-graphic-state' }, svg); status.textContent = state;
    if (renderer.kind === 'controller') {
      for (let index = 0; index < 8; index += 1) svgElement('line', { x1: 35 + index * 38, y1: 121, x2: 35 + index * 38, y2: 83, class: 'device-graphic-fader' }, svg);
      for (let index = 0; index < renderer.controls; index += 1) svgElement('circle', { cx: 35 + (index % 8) * 38, cy: 69 + Math.floor(index / 8) * 18, r: 5, class: 'device-graphic-control' }, svg);
    } else if (renderer.kind === 'pedal' || renderer.kind === 'pedalboard') {
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
    const ports = svgElement('text', { x: 24, y: 139, class: 'device-graphic-caption' }, svg); ports.textContent = renderer.kind === 'interface' ? '4 inputs · 4 outputs' : 'MIDI in · MIDI out';
    return renderer;
  }
  window.MackesDeviceRenderer = Object.freeze({
    keys: Object.freeze([...renderers.map(renderer => renderer.key), fallback.key]),
    rendererFor,
    appendGraphic,
  });
}());
