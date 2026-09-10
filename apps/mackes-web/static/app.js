const state = document.querySelector('#state');
const health = document.querySelector('#health');
const reconnectBanner = document.querySelector('#reconnect-banner');
const title = document.querySelector('#view-title');
const operation = document.querySelector('#operation');
const labels = window.MackesNavigation.labels;
let monitorPaused = false;
let monitorCleared = false;
let currentGeneration = 0;
let operationSequence = 0;
let activeView = 'state';
let viewLoadSequence = 0;
let activeViewAbortController = null;
const browserSmoke = window.location.hash.includes('browser_smoke');
let consecutiveHealthFailures = 0;
let lastHealthSuccessAt = 0;
let scheduledRefreshActive = false;
let lastSequence = 0;
const eventLog = [];
const monitorFilter = { endpoint: '', channel: '', kind: '' };
const eventTimes = [];
let dirtyForm = false;
let routeDraftDirty = false;
const themeButton = document.querySelector('#theme');
const requestedTheme = new URLSearchParams(window.location.search).get('theme')
  || new URLSearchParams(window.location.hash.slice(1)).get('theme');
const savedTheme = requestedTheme === 'light' || requestedTheme === 'dark'
  ? requestedTheme : window.localStorage.getItem('mackes-theme');
const routingCards = document.querySelector('#routing-cards');
let routeDraft = [];
const deviceBoard = document.querySelector('#device-board');
const featureBoard = document.querySelector('#feature-board');
const featureFilterLabel = document.querySelector('#feature-filter-label');
const featureFilter = document.querySelector('#feature-filter');
let featureEntries = [];
const sceneBoard = document.querySelector('#scene-board');
const capabilityBoard = document.querySelector('#capability-board');
const workspaceInspector = document.querySelector('#workspace-inspector');
const inspectorSummary = document.querySelector('#inspector-summary');
const layoutToggle = document.querySelector('#layout-toggle');
const novationGridHeading = document.querySelector('#novation-grid-heading');
const studioFlow = document.querySelector('#studio-flow');
const studioFlowGraph = document.querySelector('#studio-flow-graph');
const studioFlowStatus = document.querySelector('#studio-flow-status');
const studioFlowAdd = document.querySelector('#studio-flow-add');
function renderStudioFlow(devices) {
  if (!studioFlow || !studioFlowGraph) return;
  studioFlowGraph.querySelectorAll('[data-flow-node], [data-flow-link]').forEach(item => item.remove());
  studioFlow.hidden = !devices.length;
  if (!devices.length) {
    if (studioFlowStatus) studioFlowStatus.textContent = 'No devices found yet.';
    return;
  }
  const width = 960;
  const nodeWidth = Math.min(220, Math.max(150, Math.floor((width - 80) / devices.length) - 20));
  const gap = (width - 40 - (nodeWidth * devices.length)) / Math.max(1, devices.length - 1);
  const nodes = devices.map((device, index) => {
    const name = device.name || device.alias || 'Studio device';
    const stateValue = String(device.state || device.connection_state || 'unknown');
    return { device, name, stateValue, x: 20 + index * (nodeWidth + gap), y: 78 };
  });
  const ns = ['http', String.fromCharCode(58, 47, 47), 'www.w3.org/2000/svg'].join('');
  const addSvg = (tag, attrs, parent = studioFlowGraph) => {
    const element = document.createElementNS(ns, tag);
    Object.entries(attrs).forEach(([key, value]) => element.setAttribute(key, value));
    parent.append(element);
    return element;
  };
  nodes.slice(0, -1).forEach((node, index) => {
    const next = nodes[index + 1];
    addSvg('path', { d: `M ${node.x + nodeWidth} 150 C ${node.x + nodeWidth + gap / 2} 150, ${next.x - gap / 2} 150, ${next.x} 150`, class: 'flow-link', 'data-flow-link': 'true', 'aria-hidden': 'true' });
  });
  nodes.forEach((node, index) => {
    const group = addSvg('g', { class: 'flow-node', 'data-flow-node': 'true', tabindex: '0', role: 'button', 'aria-label': `${node.name}; ${node.stateValue}` });
    group.addEventListener('click', () => { showInspector(`${node.name} selected · ${node.stateValue}. Choose a graphical action from the workspace.`); });
    group.addEventListener('keydown', event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); group.click(); } });
    addSvg('rect', { x: node.x, y: node.y, width: nodeWidth, height: 145, rx: 8, class: 'flow-node-chassis' }, group);
    addSvg('circle', { cx: node.x + 24, cy: 104, r: 10, class: `flow-status flow-status-${node.stateValue.toLowerCase().replace(/[^a-z]+/g, '-')}` }, group);
    const titleText = addSvg('text', { x: node.x + 44, y: 112, class: 'flow-node-title' }, group); titleText.textContent = node.name;
    const stateText = addSvg('text', { x: node.x + 44, y: 135, class: 'flow-node-state' }, group); stateText.textContent = node.stateValue;
    addSvg('rect', { x: node.x + 16, y: 174, width: 12, height: 12, class: 'flow-port flow-port-in' }, group);
    addSvg('rect', { x: node.x + nodeWidth - 28, y: 174, width: 12, height: 12, class: 'flow-port flow-port-out' }, group);
    const inputText = addSvg('text', { x: node.x + 34, y: 184, class: 'flow-port-label' }, group); inputText.textContent = 'in';
    const outputText = addSvg('text', { x: node.x + nodeWidth - 36, y: 184, class: 'flow-port-label', 'text-anchor': 'end' }, group); outputText.textContent = 'out';
    if (index === 0) group.setAttribute('aria-description', 'Source device.');
    else if (index === nodes.length - 1) group.setAttribute('aria-description', 'Destination device.');
    else group.setAttribute('aria-description', 'Connected studio device.');
  });
  if (studioFlowStatus) studioFlowStatus.textContent = `${devices.length} device${devices.length === 1 ? '' : 's'} shown in the studio flow.`;
}
studioFlowAdd?.addEventListener('click', () => { showInspector('Choose a named device or endpoint to add it to the studio flow.'); operation.textContent = 'Device palette is ready for the next graphical builder.'; });
function showInspector(summary) {
  if (!workspaceInspector || !inspectorSummary) return;
  inspectorSummary.textContent = summary;
  workspaceInspector.hidden = false;
}
function presentStatus(value) {
  if (!state) return;
  if (Array.isArray(value)) {
    state.textContent = `${value.length} item${value.length === 1 ? '' : 's'} received from the daemon.`;
    return;
  }
  if (!value || typeof value !== 'object') {
    state.textContent = String(value ?? 'No live status available.');
    return;
  }
  const entries = Object.entries(value).filter(([key]) => !['content', 'bytes', 'raw', 'configuration'].includes(key));
  const meaningful = entries.slice(0, 8).map(([key, item]) => {
    const label = key.replace(/[_-]+/g, ' ').replace(/\b\w/g, character => character.toUpperCase());
    if (Array.isArray(item)) return `${label}: ${item.length} items`;
    if (item && typeof item === 'object') return `${label}: available`;
    return `${label}: ${String(item)}`;
  });
  state.textContent = meaningful.length ? meaningful.join(' · ') : 'The daemon returned no additional displayable status.';
}
function publishUiState(extra = {}) {
  window.MackesStateStore?.publish({ view: activeView, generation: currentGeneration,
    monitorPaused, monitorCleared, dirtyForm, routeDraftDirty, ...extra });
}
const viewFromLocation = () => window.MackesNavigation.viewFromLocation();
function syncViewPanels(view) {
  if (deviceControl) deviceControl.hidden = view !== 'devices';
  if (assignmentControls) assignmentControls.hidden = view !== 'mappings' && view !== 'devices';
  if (routingControls) routingControls.hidden = view !== 'routes';
  if (sceneControls) sceneControls.hidden = view !== 'scenes';
  if (monitorControls) monitorControls.hidden = view !== 'monitor';
  const systemBoard = document.querySelector('#system-board');
  if (systemBoard) systemBoard.hidden = view !== 'system';
  const recoveryActions = document.querySelector('#recovery-actions');
  if (recoveryActions) recoveryActions.hidden = false;
  const sceneActions = document.querySelector('#scene-actions');
  if (sceneActions) sceneActions.hidden = view !== 'scenes';
  const configurationActions = document.querySelector('#configuration-actions');
  if (configurationActions) configurationActions.hidden = view !== 'system';
  const hardwareActions = document.querySelector('#hardware-actions');
  if (hardwareActions) hardwareActions.hidden = view !== 'system';
}
function updateBreadcrumbs(view) {
  const breadcrumbs = document.querySelector('#breadcrumbs');
  if (!breadcrumbs) return;
  breadcrumbs.replaceChildren();
  const root = document.createElement('a');
  root.href = '/state';
  root.textContent = 'Home';
  breadcrumbs.append(root);
  const separator = document.createElement('span');
  separator.textContent = ' / ';
  separator.setAttribute('aria-hidden', 'true');
  breadcrumbs.append(separator);
  const current = document.createElement('span');
  current.textContent = labels[view] || 'Workspace';
  current.setAttribute('aria-current', 'page');
  breadcrumbs.append(current);
  if (view === 'devices' && window.location.pathname === '/devices/novation') {
    const deviceSeparator = document.createElement('span');
    deviceSeparator.textContent = ' / ';
    deviceSeparator.setAttribute('aria-hidden', 'true');
    breadcrumbs.append(deviceSeparator);
    const device = document.createElement('span');
    device.textContent = 'Novation';
    device.setAttribute('aria-current', 'page');
    breadcrumbs.append(device);
  }
}
function navigate(view, push = true) {
  if (activeView === 'routes' && view !== 'routes' && routeDraftDirty && !window.confirm('Leave Routing and discard the unsaved route draft?')) return;
  if (push) window.history.pushState({ view }, '', `/${view}`);
  window.MackesStateStore?.publish({ view, generation: currentGeneration });
  document.querySelectorAll('[data-view]').forEach(button => {
    button.setAttribute('aria-current', button.dataset.view === view ? 'page' : 'false');
  });
  syncViewPanels(view);
  updateBreadcrumbs(view);
  activeViewAbortController?.abort();
  showInspector(`${labels[view]} workspace selected. Refresh to inspect authoritative state.`);
  load(view);
}
async function boundedFetch(url, options = {}, milliseconds = 2000) {
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), milliseconds);
  const externalSignal = options.signal;
  const abortExternal = () => controller.abort();
  if (externalSignal) {
    if (externalSignal.aborted) controller.abort();
    else externalSignal.addEventListener('abort', abortExternal, { once: true });
  }
  try { return await fetch(url, { ...options, signal: controller.signal }); }
  finally {
    window.clearTimeout(timeout);
    externalSignal?.removeEventListener('abort', abortExternal);
  }
}
if (savedTheme === 'light') document.body.classList.add('light');
function updateThemeLabel() { themeButton.textContent = document.body.classList.contains('light') ? 'Use dark theme' : 'Use light theme'; }
function visibleEvents() {
  return eventLog.filter(event => (!monitorFilter.endpoint || event.endpoint === monitorFilter.endpoint)
    && (!monitorFilter.channel || String(event.channel) === monitorFilter.channel)
    && (!monitorFilter.kind || event.kind === monitorFilter.kind));
}
function updateMonitorStats() {
  const now = Date.now();
  while (eventTimes.length && eventTimes[0] < now - 60000) eventTimes.shift();
  document.querySelector('#monitor-stats').textContent = `Event rate: ${eventTimes.length}/min · browser buffer: ${eventLog.length}/256 (presentation only)`;
}
function routeListFromBody(body) {
  if (Array.isArray(body)) return body;
  return Array.isArray(body?.routes) ? body.routes : [];
}
function renderDeviceBoard(body) {
  const devices = Array.isArray(body) ? body : (body?.endpoints || body?.devices || []);
  renderStudioFlow(devices);
  deviceBoard.replaceChildren();
  const endpointOptions = document.querySelector('#device-destination'); endpointOptions.replaceChildren();
  devices.forEach(device => { const option = document.createElement('option'); option.value = device.id || device.name || device.alias || ''; option.label = device.name || device.alias || option.value; if (option.value) endpointOptions.append(option); });
  if (!devices.length) { deviceBoard.hidden = true; return; }
  deviceBoard.hidden = false;
  devices.forEach((device, index) => {
    const card = document.createElement('article'); card.className = 'device-card';
    const name = device.name || device.alias || device.id || `Endpoint ${index + 1}`;
    const title = document.createElement('h3'); title.textContent = name; card.append(title);
    window.MackesDeviceRenderer?.appendGraphic(device, card);
    if (/novation|launch control/i.test(name)) {
      const editorLink = document.createElement('a'); editorLink.href = '/devices/novation'; editorLink.textContent = 'Open Novation editor'; editorLink.className = 'device-editor-link'; card.append(editorLink);
    }
    const details = document.createElement('dl');
    [['State', device.state || device.connection_state || 'unknown'], ['Transport', device.transport || device.kind || 'unspecified'], ['Direction', device.direction || 'unspecified'], ['Address', device.address || device.logical_port || device.port || 'stable identity pending']].forEach(([label, value]) => {
      const term = document.createElement('dt'); term.textContent = label; const description = document.createElement('dd'); description.textContent = String(value); details.append(term, description);
    });
    card.append(details);
    const capabilities = device.capabilities || device.features || device.addressable_features;
    if (Array.isArray(capabilities) && capabilities.length) {
      const featureList = document.createElement('ul'); featureList.className = 'device-features';
      capabilities.forEach(feature => { const item = document.createElement('li'); item.textContent = typeof feature === 'string' ? feature : (feature.label || feature.id || feature.name || 'Qualified feature'); featureList.append(item); });
      card.append(featureList);
    }
    deviceBoard.append(card);
  });
  renderFeatureBoard(devices);
}
function renderFeatureBoard(devices) {
  const catalog = window.MackesFeatureCatalog?.catalog || [
    { match: /novation|launch control/i, name: 'Novation Launch Control XL', profile: 'novation.launch-control-xl', source: 'Qualified profile + Programmer Reference', features: ['24 knobs', '16 channel buttons', '8 utility controls', '8 faders', 'LED intent / delivery state', 'Template and reconnect diagnostics'], operations: [{ label: 'Resync device (confirmed)', operation: 'rescan' }] },
    { match: /eventide|micropitch/i, name: 'Eventide MicroPitch Delay', profile: 'eventide.micropitch', source: 'MicroPitch QRG + Eventide profile', features: [{ label: 'Expression', control: 'expression', cc: 4 }, { label: 'Tap trigger', control: 'tap', cc: 9 }, { label: 'Active / bypass', control: 'active', cc: 14 }, { label: 'FLEX', control: 'flex', cc: 15 }, { label: 'Mix', control: 'mix', cc: 20 }, { label: 'Pitch A', control: 'pitch-a', cc: 21 }, { label: 'Pitch B', control: 'pitch-b', cc: 22 }, { label: 'Depth', control: 'depth', cc: 23 }, { label: 'Rate sensitivity', control: 'rate-sensitivity', cc: 24 }, { label: 'Pitch mix', control: 'pitch-mix', cc: 25 }, { label: 'Tone', control: 'tone', cc: 26 }, { label: 'Delay A', control: 'delay-a', cc: 27 }, { label: 'Delay B', control: 'delay-b', cc: 28 }, { label: 'Modulation', control: 'modulation', cc: 29 }, { label: 'Feedback', control: 'feedback', cc: 30 }, { label: 'Output level', control: 'output-level', cc: 31 }] },
    { match: /lexicon|reflex/i, name: 'Lexicon Reflex', profile: 'lexicon.reflex', source: 'Reflex MIDI implementation + codec metadata', features: [{ label: 'Algorithm selector', control: 'algorithm-select' }, { label: 'Algorithm parameters', control: 'parameter' }, { label: 'Echo Rhythm', control: 'echo-rhythm' }, { label: 'MIDI patch 1–4', control: 'midi-patch' }, { label: 'Register read / recall', control: 'register-recall' }, { label: 'Register store (persistent)', control: 'register-store' }, { label: 'Setup / dump diagnostics', control: 'setup-dump' }, { label: 'System reset (confirmed)', control: 'system-reset' }, { label: 'Bypass / system task', control: 'bypass-task' }] },
    { match: /pipedal/i, name: 'PiPedal', profile: 'pipedal', source: 'Pinned server operation audit', features: ['Dynamic plugin controls', 'Snapshots and presets', 'MIDI bindings', 'Levels and pedalboard state', 'Unsupported operation reasons'] },
    { match: /midisport|m-audio/i, name: 'M-Audio MIDISPORT 4x4', profile: 'm-audio.midisport-4x4', source: 'Manufacturer capability evidence', features: ['4 MIDI inputs', '4 MIDI outputs', 'Per-port activity', 'Direction-aware aliases', 'Firmware and cable state separation'] },
    { match: /rtp|apple midi/i, name: 'RTP-MIDI session', profile: 'rtp-midi', source: 'AppleMIDI/RFC session requirements', features: ['Peer identity and allowlist', 'Invitation/handshake lifecycle', 'Sequence and reorder health', 'Reconnect backoff', 'Message delivery counters'] },
    { match: /midi through|generic midi/i, name: 'Generic MIDI transport', profile: 'generic-midi', source: 'Qualified MIDI engine and endpoint schemas', features: ['Direction-aware ports', 'Channel and message filters', 'CC/note/program/pressure/bend', 'SysEx and realtime predicates', 'Stable identity and reconnect repair'] }
  ];
  featureBoard.replaceChildren();
  featureEntries = window.MackesFeatureCatalog?.entriesFor
    ? window.MackesFeatureCatalog.entriesFor(devices)
    : catalog.map(item => ({ ...item, connected: devices.some(device => item.match.test(String(device.name || device.alias || device.id || ''))) }));
  featureFilterLabel.hidden = !featureEntries.length;
  featureBoard.hidden = !featureEntries.length;
  renderFilteredFeatures();
}
function renderFilteredFeatures() {
  const query = featureFilter.value.trim().toLowerCase();
  const entries = window.MackesFeatureRenderer?.filter
    ? window.MackesFeatureRenderer.filter(featureEntries, query)
    : featureEntries.filter(item => !query || `${item.name} ${item.source}`.toLowerCase().includes(query));
  featureBoard.replaceChildren();
  if (!entries.length && featureEntries.length) { const empty = document.createElement('p'); empty.className = 'empty-state'; empty.textContent = 'No qualified product features match this filter.'; featureBoard.append(empty); return; }
  entries.forEach(item => {
    const card = document.createElement('article'); card.className = 'feature-card';
    const heading = document.createElement('h3'); heading.textContent = item.name; card.append(heading);
    const state = document.createElement('p'); state.className = item.connected ? 'feature-connected' : 'feature-disconnected'; state.textContent = item.connected ? 'Connected in live inventory' : 'Researched product · not connected'; card.append(state);
    window.MackesDeviceRenderer?.appendGraphic({ name: item.name, state: item.connected ? 'connected' : 'not connected' }, card);
    const source = document.createElement('p'); source.className = 'feature-source'; source.textContent = `Evidence: ${item.source}`; card.append(source);
    const list = document.createElement('ul'); item.features.forEach(feature => {
      const li = document.createElement('li');
      const label = typeof feature === 'string' ? feature : `${feature.label}${feature.cc === undefined ? '' : ` (CC ${feature.cc})`}`;
      if (typeof feature === 'string') li.textContent = label;
      else { const select = document.createElement('button'); select.type = 'button'; select.textContent = label; select.disabled = !item.connected; select.title = item.connected ? 'Select this qualified feature' : 'Unavailable: connect the qualified device first'; select.setAttribute('aria-description', item.connected ? (feature.cc === undefined ? 'Readback and value domain are unknown until the authoritative profile supplies them.' : `MIDI CC ${feature.cc}; value range 0 to 127.`) : 'Unavailable until the qualified device is connected.'); select.addEventListener('click', () => { deviceControl.hidden = false; configureDeviceControl(item.profile, feature.control); operation.textContent = `${item.name} ${label} selected; verify destination and value before sending.`; showInspector(`${item.name} · ${label} selected${feature.cc === undefined ? ' · readback/domain unknown' : ` · MIDI CC ${feature.cc}, range 0–127`}. Values and delivery remain governed by the live device state.`); }); li.append(select); }
      list.append(li);
    }); card.append(list);
    if (Array.isArray(item.operations) && item.operations.length) {
      const divider = document.createElement('h4'); divider.textContent = 'Implemented operations'; divider.className = 'feature-operation-divider'; card.append(divider);
      const operations = document.createElement('ul'); operations.className = 'feature-operations';
      item.operations.forEach(entry => {
        const action = document.createElement('button'); action.type = 'button'; action.textContent = entry.label; action.disabled = !item.connected;
        action.title = item.connected ? 'Forward the confirmed operation to the daemon' : 'Unavailable until the qualified device is connected';
        action.addEventListener('click', () => { if (item.connected) runOperation(entry.operation); });
        const line = document.createElement('li'); line.append(action); operations.append(line);
      });
      card.append(operations);
    }
    const edit = document.createElement('button'); edit.type = 'button'; edit.textContent = 'Open guarded control editor';
    edit.disabled = !item.connected;
    if (!item.connected) edit.textContent = 'Connect device to edit';
    edit.addEventListener('click', () => {
      if (!item.connected) return;
      deviceControl.hidden = false;
      if (item.profile === 'pipedal') { document.querySelector('#pipedal-refresh').focus(); operation.textContent = 'PiPedal workspace ready; refresh the authoritative plugin and operation catalog before choosing an operation.'; }
      else { configureDeviceControl(item.profile, ''); operation.textContent = `${item.name} editor ready; choose a control, channel, value, and destination before sending.`; }
    });
    card.append(edit);
    featureBoard.append(card);
  });
}
function configureDeviceControl(profile, control) {
  document.querySelector('#device-profile').value = profile;
  const controls = {
    'novation.launch-control-xl': ['Knob', 'Channel button', 'Utility control', 'Fader'],
    'eventide.micropitch': ['Expression', 'Tap trigger', 'Active / bypass', 'FLEX', 'Mix', 'Pitch A', 'Pitch B', 'Depth', 'Tone', 'Delay A', 'Delay B', 'Feedback', 'Output level'],
    'lexicon.reflex': ['Algorithm', 'Parameter', 'Echo Rhythm', 'MIDI patch', 'Register recall', 'Register store', 'Bypass'],
    pipedal: ['Plugin control', 'Snapshot', 'Preset', 'Pedalboard item'],
  }[profile] || [];
  const controlChoice = document.querySelector('#device-control-name');
  controlChoice.replaceChildren(new Option('Choose a device control', ''));
  controls.forEach(value => controlChoice.add(new Option(value, value.toLowerCase().replace(/[^a-z0-9]+/g, '-'))));
  controlChoice.value = control || '';
  const reset = profile === 'lexicon.reflex' && control === 'system-reset';
  for (const id of ['device-channel', 'device-value']) {
    const input = document.querySelector(`#${id}`);
    input.required = !reset;
    input.closest('label').hidden = reset;
  }
  if (!reset) document.querySelector('#device-value').value = 0;
  document.querySelector('#device-value-display').textContent = document.querySelector('#device-value').value;
  document.querySelector('#device-control-name').focus();
}
document.querySelector('#device-value').addEventListener('input', event => { document.querySelector('#device-value-display').textContent = event.target.value; });
featureFilter.addEventListener('input', renderFilteredFeatures);
function renderCapabilityBoard(body) {
  capabilityBoard.replaceChildren();
  const operations = body?.operations && typeof body.operations === 'object' ? Object.entries(body.operations) : [];
  capabilityBoard.hidden = !operations.length;
  if (!operations.length) return;
  const heading = document.createElement('h3'); heading.textContent = 'Platform capability coverage'; capabilityBoard.append(heading);
  const list = document.createElement('ul');
  operations.forEach(([name, status]) => { const item = document.createElement('li'); item.className = String(status).includes('partial') ? 'partial' : 'implemented'; item.textContent = `${name}: ${String(status).replaceAll('_', ' ')}`; item.tabIndex = 0; const inspect = () => showInspector(`Capability ${name}: ${String(status).replaceAll('_', ' ')}. Refresh to inspect authoritative state.`); item.addEventListener('click', inspect); item.addEventListener('keydown', event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); inspect(); } }); list.append(item); });
  capabilityBoard.append(list);
  const unsupported = body?.unsupported?.remaining_mutations;
  if (unsupported) { const note = document.createElement('p'); note.className = 'capability-gap'; note.textContent = `Remaining mutation coverage: ${unsupported}`; capabilityBoard.append(note); }
}
function renderSystemBoard(body) {
  const board = document.querySelector('#system-board'); board.replaceChildren();
  const sections = [['Service', body?.service], ['Web API', body?.web], ['Recovery', body?.recovery_catalog], ['RTP-MIDI', body?.rtp_midi]];
  sections.forEach(([name, values]) => {
    if (!values || typeof values !== 'object') return;
    const card = document.createElement('article'); card.className = 'feature-card';
    const heading = document.createElement('h3'); heading.textContent = name; card.append(heading);
    const list = document.createElement('dl'); Object.entries(values).forEach(([key, value]) => {
      const term = document.createElement('dt'); term.textContent = key.replaceAll('_', ' ').replace(/\b\w/g, character => character.toUpperCase());
      const detail = document.createElement('dd'); detail.textContent = Array.isArray(value) ? `${value.length} items` : (value && typeof value === 'object' ? 'Available' : String(value)); list.append(term, detail);
    }); card.append(list); board.append(card);
  });
  board.hidden = !board.children.length;
}
function renderSceneBoard(body) {
  const scenes = Array.isArray(body?.scenes) ? body.scenes : [];
  const catalog = body?.catalog || {};
  const setlists = Array.isArray(catalog.setlists) ? catalog.setlists : [];
  const projects = Array.isArray(catalog.projects) ? catalog.projects : [];
  const activeScene = body?.active_scene || body?.activeScene || '';
  sceneBoard.replaceChildren(); sceneBoard.hidden = false;
  const createSetlist = document.createElement('button'); createSetlist.type = 'button'; createSetlist.textContent = 'Create empty setlist';
  createSetlist.addEventListener('click', async () => {
    const id = window.prompt('New setlist ID (1–96 characters):');
    if (!id || id.length > 96) { operation.textContent = 'Setlist creation cancelled: ID must be 1–96 characters.'; return; }
    operation.dataset.state = 'pending'; operation.setAttribute('aria-busy', 'true'); operation.textContent = `Creating setlist ${id}…`;
    try {
      const response = await fetch('/api/v1/scenes', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ setlist_create: { id, projects: [] } }) });
      const result = await response.json().catch(() => ({}));
      operation.textContent = response.ok ? `Setlist ${result.setlist || id} created.` : `Setlist creation failed (${response.status})`;
      if (response.ok) await load('scenes');
    } catch (error) { operation.dataset.state = 'unknown'; operation.textContent = `Setlist creation outcome unknown; inspect authoritative state before retrying (${error})`; }
    finally { if (operation.dataset.state === 'pending') operation.dataset.state = 'complete'; operation.setAttribute('aria-busy', 'false'); }
  });
  sceneBoard.append(createSetlist);
  if (!(scenes.length || setlists.length || projects.length)) {
    const empty = document.createElement('p'); empty.className = 'empty-state';
    empty.textContent = 'No authoritative scenes, setlists, or projects are configured.';
    sceneBoard.append(empty);
  }
  scenes.forEach((scene, index) => {
    const card = document.createElement('article'); card.className = 'scene-card';
    const id = typeof scene === 'string' ? scene : (scene.id || `scene-${index + 1}`);
    if (id === activeScene) { card.classList.add('active-scene'); card.setAttribute('aria-current', 'true'); }
    const title = document.createElement('h3'); title.textContent = typeof scene === 'string' ? scene : (scene.name || id); card.append(title);
    const meta = document.createElement('p'); meta.textContent = `${scene.category || 'Scene'} · ${Array.isArray(scene.actions) ? scene.actions.length : 0} actions`; card.append(meta);
    if (Array.isArray(scene.actions) && scene.actions.length) {
      const actions = document.createElement('ul'); actions.className = 'scene-actions';
      scene.actions.slice(0, 3).forEach(action => { const item = document.createElement('li'); item.textContent = typeof action === 'string' ? action : (action.description || action.operation || action.target || 'Configured action'); actions.append(item); });
      if (scene.actions.length > 3) { const item = document.createElement('li'); item.textContent = `+${scene.actions.length - 3} more`; actions.append(item); }
      card.append(actions);
    }
    appendSceneActionBuilder(card, id, Array.isArray(scene.actions) ? scene.actions : []);
    const select = document.createElement('button'); select.type = 'button'; select.textContent = 'Select scene'; select.dataset.sceneId = id; select.addEventListener('click', () => { document.querySelector('#scene-id').value = id; showInspector(`Scene ${id} selected. Refresh to inspect authoritative state.`); }); card.append(select);
    sceneBoard.append(card);
  });
  [...setlists.map(item => ['Setlist', item]), ...projects.map(item => ['Project', item])].forEach(([kind, entry]) => {
    const card = document.createElement('article'); card.className = 'scene-card';
    const id = typeof entry === 'string' ? entry : (entry.id || entry.name || '');
    const title = document.createElement('h3'); title.textContent = id || kind; card.append(title);
    const meta = document.createElement('p'); meta.textContent = `${kind} · authoritative catalog entry`; card.append(meta);
    if (kind === 'Setlist' && id) {
      const preview = document.createElement('button'); preview.type = 'button'; preview.textContent = 'Preview recall plan (dry run)';
      preview.addEventListener('click', async () => {
        operation.dataset.state = 'pending'; operation.setAttribute('aria-busy', 'true');
        operation.textContent = `Setlist ${id} dry-run pending…`;
        try {
          const response = await fetch('/api/v1/scenes', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ preview_setlist: id }) });
          const body = await response.json().catch(() => ({}));
          const planned = Array.isArray(body.preview?.projects) ? body.preview.projects : [];
          const counts = planned.map(project => `${project.id}: ${Array.isArray(project.scenes) ? project.scenes.length : 0} scenes`).join(', ');
          operation.textContent = response.ok ? `Dry-run only; no scene selected or executed. ${counts || 'No projects in setlist.'}` : `Setlist preview failed (${response.status})`;
        } catch (error) { operation.dataset.state = 'unknown'; operation.textContent = `Setlist preview outcome unknown; inspect authoritative state before retrying (${error})`; }
        finally { if (operation.dataset.state === 'pending') operation.dataset.state = 'complete'; operation.setAttribute('aria-busy', 'false'); }
      });
      card.append(preview);
      const copy = document.createElement('button'); copy.type = 'button'; copy.textContent = 'Copy setlist';
      copy.addEventListener('click', async () => {
        const newId = window.prompt('New setlist ID (1–96 characters):', `${id}-copy`);
        if (!newId || newId.length > 96) { operation.textContent = 'Setlist copy cancelled: ID must be 1–96 characters.'; return; }
        operation.dataset.state = 'pending'; operation.setAttribute('aria-busy', 'true');
        operation.textContent = `Copying setlist ${id}…`;
        try {
          const response = await fetch('/api/v1/scenes', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ setlist_copy: { source: id, new_id: newId } }) });
          const body = await response.json().catch(() => ({}));
          operation.textContent = response.ok ? `Setlist copied as ${body.setlist || newId}.` : `Setlist copy failed (${response.status})`;
          if (response.ok) await load('scenes');
        } catch (error) { operation.dataset.state = 'unknown'; operation.textContent = `Setlist copy outcome unknown; inspect authoritative state before retrying (${error})`; }
        finally { if (operation.dataset.state === 'pending') operation.dataset.state = 'complete'; operation.setAttribute('aria-busy', 'false'); }
      });
      card.append(copy);
      const exportProject = document.createElement('button'); exportProject.type = 'button'; exportProject.textContent = 'Export project';
      exportProject.addEventListener('click', () => {
        const payload = JSON.stringify({ schema_version: 1, project: entry }, null, 2);
        const link = document.createElement('a'); link.href = URL.createObjectURL(new Blob([payload], { type: 'application/json' })); link.download = `${id}.project.json`; link.click();
        setTimeout(() => URL.revokeObjectURL(link.href), 0);
        operation.dataset.state = 'complete'; operation.textContent = `Project ${id} exported from authoritative state.`;
      });
      card.append(exportProject);
      const remove = document.createElement('button'); remove.type = 'button'; remove.textContent = 'Delete setlist';
      remove.addEventListener('click', async () => {
        if (!window.confirm(`Delete setlist ${id}? This does not delete its projects.`)) return;
        operation.dataset.state = 'pending'; operation.setAttribute('aria-busy', 'true');
        operation.textContent = `Deleting setlist ${id}…`;
        try {
          const response = await fetch('/api/v1/scenes', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ setlist_delete: id }) });
          const body = await response.json().catch(() => ({}));
          operation.textContent = response.ok ? `Setlist ${body.setlist || id} deleted.` : `Setlist deletion failed (${response.status})`;
          if (response.ok) await load('scenes');
        } catch (error) { operation.dataset.state = 'unknown'; operation.textContent = `Setlist deletion outcome unknown; inspect authoritative state before retrying (${error})`; }
        finally { if (operation.dataset.state === 'pending') operation.dataset.state = 'complete'; operation.setAttribute('aria-busy', 'false'); }
      });
      card.append(remove);
      const reorder = document.createElement('button'); reorder.type = 'button'; reorder.textContent = 'Reorder projects';
      reorder.addEventListener('click', async () => {
        const current = Array.isArray(entry.projects) ? entry.projects.join(', ') : '';
        const text = window.prompt('Project IDs in recall order (comma-separated):', current);
        if (text === null) return;
        const projects = text.split(',').map(value => value.trim()).filter(Boolean);
        if (projects.some(value => value.length > 96) || projects.length > 128) { operation.textContent = 'Setlist reorder cancelled: IDs or project count exceed bounds.'; return; }
        operation.dataset.state = 'pending'; operation.setAttribute('aria-busy', 'true'); operation.textContent = `Reordering setlist ${id}…`;
        try {
          const response = await fetch('/api/v1/scenes', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ setlist: { id, projects } }) });
          const body = await response.json().catch(() => ({}));
          operation.textContent = response.ok ? `Setlist ${body.setlist || id} order saved.` : `Setlist reorder failed (${response.status})`;
          if (response.ok) await load('scenes');
        } catch (error) { operation.dataset.state = 'unknown'; operation.textContent = `Setlist reorder outcome unknown; inspect authoritative state before retrying (${error})`; }
        finally { if (operation.dataset.state === 'pending') operation.dataset.state = 'complete'; operation.setAttribute('aria-busy', 'false'); }
      });
      card.append(reorder);
      const exportSetlist = document.createElement('button'); exportSetlist.type = 'button'; exportSetlist.textContent = 'Export setlist';
      exportSetlist.addEventListener('click', () => {
        const payload = JSON.stringify({ schema_version: 1, setlist: { id, projects: Array.isArray(entry.projects) ? entry.projects : [] } }, null, 2);
        const link = document.createElement('a'); link.href = URL.createObjectURL(new Blob([payload], { type: 'application/json' })); link.download = `${id}.setlist.json`; link.click();
        setTimeout(() => URL.revokeObjectURL(link.href), 0);
        operation.dataset.state = 'complete'; operation.textContent = `Setlist ${id} exported from authoritative state.`;
      });
      card.append(exportSetlist);
      const importSetlist = document.createElement('button'); importSetlist.type = 'button'; importSetlist.textContent = 'Import setlist';
      const importFile = document.createElement('input'); importFile.type = 'file'; importFile.accept = 'application/json,.json'; importFile.hidden = true;
      importSetlist.addEventListener('click', () => importFile.click());
      importFile.addEventListener('change', async () => {
        const file = importFile.files && importFile.files[0]; importFile.value = ''; if (!file) return;
        try {
          if (file.size > 128 * 1024) { operation.dataset.state = 'error'; operation.textContent = 'Setlist import rejected: file exceeds 128 KiB limit.'; return; }
          const parsed = JSON.parse(await file.text()); const imported = parsed && parsed.setlist;
          const projects = imported && Array.isArray(imported.projects) ? imported.projects : null;
          if (!projects || projects.length > 128 || projects.some(project => typeof project !== 'string' || !project || project.length > 96)) { operation.dataset.state = 'error'; operation.textContent = 'Setlist import rejected: invalid project list or bounds.'; return; }
          if (!window.confirm(`Replace projects in setlist ${id} with imported ordering?`)) return;
          operation.dataset.state = 'pending'; operation.setAttribute('aria-busy', 'true'); operation.textContent = `Importing setlist ${id}…`;
          const response = await fetch('/api/v1/scenes', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ setlist: { id, projects } }) });
          const body = await response.json().catch(() => ({})); operation.textContent = response.ok ? `Setlist ${body.setlist || id} imported.` : `Setlist import failed (${response.status})`;
          if (response.ok) await load('scenes');
        } catch (error) { operation.dataset.state = 'unknown'; operation.textContent = `Setlist import outcome unknown; inspect authoritative state before retrying (${error})`; }
        finally { if (operation.dataset.state === 'pending') operation.dataset.state = 'complete'; operation.setAttribute('aria-busy', 'false'); }
      });
      card.append(importSetlist, importFile);
    } else if (kind === 'Project' && id) {
      const copy = document.createElement('button'); copy.type = 'button'; copy.textContent = 'Copy project';
      copy.addEventListener('click', async () => {
        const newId = window.prompt('New project ID (1–96 characters):', `${id}-copy`);
        if (!newId || newId.length > 96) { operation.textContent = 'Project copy cancelled: ID must be 1–96 characters.'; return; }
        operation.dataset.state = 'pending'; operation.setAttribute('aria-busy', 'true');
        operation.textContent = `Copying project ${id}…`;
        try {
          const response = await fetch('/api/v1/scenes', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ project_copy: { source: id, new_id: newId } }) });
          const body = await response.json().catch(() => ({}));
          operation.textContent = response.ok ? `Project copied as ${body.project || newId}.` : `Project copy failed (${response.status})`;
          if (response.ok) await load('scenes');
        } catch (error) { operation.dataset.state = 'unknown'; operation.textContent = `Project copy outcome unknown; inspect authoritative state before retrying (${error})`; }
        finally { if (operation.dataset.state === 'pending') operation.dataset.state = 'complete'; operation.setAttribute('aria-busy', 'false'); }
      });
      card.append(copy);
    }
    sceneBoard.append(card);
  });
}
function appendSceneActionBuilder(card, sceneId, sourceActions) {
  const builder = document.createElement('details'); builder.className = 'scene-action-builder';
  const summary = document.createElement('summary'); summary.textContent = 'Edit actions visually'; builder.append(summary);
  const list = document.createElement('div'); list.className = 'visual-action-list';
  const actions = sourceActions.map(action => typeof action === 'object' && action ? { ...action } : { description: String(action), operation: 'note' });
  const namedTargets = routeEndpointCatalog.filter(endpoint => endpoint?.id != null).map(endpoint => ({ id: String(endpoint.id), name: endpoint.name || `Named device ${endpoint.id}` }));
  const render = () => {
    list.replaceChildren();
    actions.forEach((action, index) => {
      const item = document.createElement('article'); item.className = 'visual-action-card';
      const heading = document.createElement('strong'); heading.textContent = `Action ${index + 1}`; item.append(heading);
      const operationField = document.createElement('label'); operationField.textContent = 'Do';
      const operationChoice = document.createElement('select');
      [['set', 'Set a value'], ['select', 'Select a preset'], ['enable', 'Enable'], ['bypass', 'Bypass'], ['wait', 'Wait'], ['note', 'Show a note']].forEach(([value, label]) => operationChoice.add(new Option(label, value)));
      operationChoice.value = action.operation || 'note'; operationChoice.addEventListener('change', () => { action.operation = operationChoice.value; }); operationField.append(operationChoice); item.append(operationField);
      const targetField = document.createElement('label'); targetField.textContent = 'On';
      const targetChoice = document.createElement('select'); targetChoice.add(new Option('Choose a named device', ''));
      namedTargets.forEach(target => targetChoice.add(new Option(target.name, target.id)));
      targetChoice.value = action.target == null ? '' : String(action.target); targetChoice.addEventListener('change', () => { action.target = targetChoice.value; }); targetField.append(targetChoice); item.append(targetField);
      const valueField = document.createElement('label'); valueField.textContent = action.operation === 'wait' ? 'Seconds' : 'Value';
      const valueInput = document.createElement('input'); valueInput.type = 'number'; valueInput.min = '0'; valueInput.max = action.operation === 'wait' ? '60' : '16383'; valueInput.step = 'any'; valueInput.value = action.value == null ? '0' : action.value; valueInput.addEventListener('input', () => { action.value = Number(valueInput.value); }); valueField.append(valueInput); item.append(valueField);
      const moveUp = document.createElement('button'); moveUp.type = 'button'; moveUp.textContent = 'Move earlier'; moveUp.disabled = index === 0; moveUp.addEventListener('click', () => { [actions[index - 1], actions[index]] = [actions[index], actions[index - 1]]; render(); }); item.append(moveUp);
      const moveDown = document.createElement('button'); moveDown.type = 'button'; moveDown.textContent = 'Move later'; moveDown.disabled = index === actions.length - 1; moveDown.addEventListener('click', () => { [actions[index], actions[index + 1]] = [actions[index + 1], actions[index]]; render(); }); item.append(moveDown);
      const remove = document.createElement('button'); remove.type = 'button'; remove.className = 'scene-action-remove'; remove.textContent = 'Remove'; remove.addEventListener('click', () => { actions.splice(index, 1); render(); }); item.append(remove);
      list.append(item);
    });
    if (!actions.length) { const empty = document.createElement('p'); empty.className = 'empty-state'; empty.textContent = 'No actions yet. Add a visual action to this scene.'; list.append(empty); }
  };
  render(); builder.append(list);
  const add = document.createElement('button'); add.type = 'button'; add.textContent = 'Add visual action'; add.addEventListener('click', () => { actions.push({ operation: 'set', target: '', value: 0 }); render(); }); builder.append(add);
  const save = document.createElement('button'); save.type = 'button'; save.textContent = 'Save visual actions'; save.addEventListener('click', async () => {
    operation.dataset.state = 'pending'; operation.setAttribute('aria-busy', 'true'); operation.textContent = `Saving visual actions for ${sceneId}…`;
    try {
      const response = await fetch('/api/v1/scenes', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ scene: sceneId, actions }) });
      const result = await response.json().catch(() => ({}));
      operation.textContent = response.ok ? `Visual actions for ${sceneId} saved.` : `Scene action save failed (${response.status}): ${result.error || 'daemon rejected the action list'}`;
      if (response.ok) await load('scenes');
    } catch (error) { operation.dataset.state = 'unknown'; operation.textContent = `Scene action outcome unknown; inspect authoritative state before retrying (${error})`; }
    finally { if (operation.dataset.state === 'pending') operation.dataset.state = 'complete'; operation.setAttribute('aria-busy', 'false'); }
  }); builder.append(save); card.append(builder);
}
function renderRoutingBoard(routes) {
  routingCards.replaceChildren();
  if (!routes.length) {
    const empty = document.createElement('p'); empty.className = 'empty-state'; empty.textContent = 'No connections yet. Add one to begin.'; routingCards.append(empty); return;
  }
  routes.forEach((route, index) => {
    const card = document.createElement('article'); card.className = 'route-card'; card.dataset.routeIndex = index;
    card.dataset.route = JSON.stringify(route);
    const header = document.createElement('div'); header.className = 'route-card-header';
    const title = document.createElement('strong'); title.textContent = `Connection ${index + 1}`;
    const remove = document.createElement('button'); remove.className = 'route-remove'; remove.type = 'button'; remove.dataset.removeRoute = index; remove.textContent = 'Remove';
    header.append(title, remove); card.append(header);
    [['source', 'Source'], ['destination', 'Destination']].forEach(([key, label]) => {
      const field = document.createElement('label'); field.textContent = label;
      const input = document.createElement('select'); input.dataset.routeKey = key;
      const direction = key === 'source' ? 'input' : 'output';
      const choices = routeEndpointCatalog.filter(endpoint => !endpoint.direction || endpoint.direction === direction);
      if (choices.length) choices.forEach(endpoint => input.add(new Option(`${endpoint.name} (${endpoint.direction})`, endpoint.id)));
      else input.add(new Option('Refresh endpoints to choose a named port', ''));
      const current = Number.isFinite(route[key]) ? String(route[key]) : '';
      if (current && !choices.some(endpoint => String(endpoint.id) === current)) input.add(new Option(`Current endpoint ${current}`, current));
      input.value = Number.isFinite(route[key]) ? String(route[key]) : '';
      field.append(input); card.append(field);
    });
    const priority = document.createElement('label'); priority.textContent = 'Priority';
    const priorityInput = document.createElement('input'); priorityInput.type = 'number'; priorityInput.min = '-32768'; priorityInput.max = '32767'; priorityInput.value = Number.isInteger(route.priority) ? route.priority : 0; priorityInput.dataset.routeKey = 'priority'; priority.append(priorityInput); card.append(priority);
    const curve = document.createElement('label'); curve.textContent = 'Curve';
    const curveSelect = document.createElement('select'); curveSelect.dataset.routeKey = 'curve'; ['linear', 'inverse', 'square', 'exponential', 'logarithmic'].forEach(value => curveSelect.add(new Option(value, value))); curveSelect.value = route.curve || 'linear'; curve.append(curveSelect); card.append(curve);
    const enabled = document.createElement('label'); enabled.className = 'route-enabled';
    const checkbox = document.createElement('input'); checkbox.type = 'checkbox'; checkbox.checked = route.enabled !== false; checkbox.dataset.routeKey = 'enabled'; enabled.append(checkbox, document.createTextNode(' Connection enabled')); card.append(enabled);
    const cycle = document.createElement('label'); cycle.className = 'route-enabled';
    const cycleInput = document.createElement('input'); cycleInput.type = 'checkbox'; cycleInput.checked = route.allow_cycle === true; cycleInput.dataset.routeKey = 'allow_cycle'; cycle.append(cycleInput, document.createTextNode(' Allow cycles')); card.append(cycle);
    const predicates = document.createElement('fieldset'); predicates.className = 'route-advanced route-condition-builder'; predicates.dataset.routeKey = 'predicates'; predicates.dataset.predicates = JSON.stringify(route.predicates || []);
    const legend = document.createElement('legend'); legend.textContent = 'Conditions'; predicates.append(legend);
    renderPredicateBuilder(predicates, route.predicates || []);
    card.append(predicates);
    routingCards.append(card);
  });
}
function predicateDescription(predicate) {
  if (predicate?.NumberRange) return `Message number ${predicate.NumberRange.minimum}–${predicate.NumberRange.maximum}`;
  if (predicate?.ValueRange) return `Message value ${predicate.ValueRange.minimum}–${predicate.ValueRange.maximum}`;
  if (predicate?.Realtime) return `Realtime ${predicate.Realtime}`;
  if (predicate?.SysExMask) return 'Qualified SysEx mask (preserved)';
  return 'Unsupported condition (preserved)';
}
function renderPredicateBuilder(container, predicates) {
  const list = document.createElement('div'); list.className = 'predicate-chips';
  predicates.forEach((predicate, index) => {
    const chip = document.createElement('div'); chip.className = 'predicate-chip';
    const text = document.createElement('span'); text.textContent = predicateDescription(predicate); chip.append(text);
    if (predicate?.NumberRange || predicate?.ValueRange) {
      const kind = predicate.NumberRange ? 'NumberRange' : 'ValueRange';
      const range = predicate[kind];
      [['minimum', 'From'], ['maximum', 'To']].forEach(([key, label]) => {
        const input = document.createElement('input'); input.type = 'number'; input.min = '0'; input.max = kind === 'NumberRange' ? '127' : '16383'; input.value = range[key]; input.dataset.predicateIndex = index; input.dataset.predicateKey = `${kind}.${key}`; input.setAttribute('aria-label', `${label} ${kind === 'NumberRange' ? 'message number' : 'message value'}`); chip.append(input);
      });
    }
    const remove = document.createElement('button'); remove.type = 'button'; remove.className = 'predicate-remove'; remove.dataset.removePredicate = index; remove.textContent = 'Remove'; remove.setAttribute('aria-label', `Remove condition ${index + 1}`); chip.append(remove);
    list.append(chip);
  });
  const actions = document.createElement('div'); actions.className = 'predicate-actions';
  [['NumberRange', 'message number range'], ['ValueRange', 'message value range'], ['Realtime', 'realtime message']].forEach(([kind, label]) => {
    const add = document.createElement('button'); add.type = 'button'; add.dataset.addPredicate = kind; add.textContent = `Add ${label}`; actions.append(add);
  });
  if (!predicates.length) { const empty = document.createElement('p'); empty.className = 'predicate-empty'; empty.textContent = 'No conditions: this connection accepts its selected message types.'; list.append(empty); }
  container.append(list, actions);
}
function refreshPredicateData(card) {
  const container = card.querySelector('[data-route-key="predicates"]');
  if (!container) return;
  const predicates = JSON.parse(container.dataset.predicates || '[]');
  container.querySelectorAll('[data-predicate-key]').forEach(input => {
    const [kind, key] = input.dataset.predicateKey.split('.');
    if (predicates[input.dataset.predicateIndex]?.[kind]) predicates[input.dataset.predicateIndex][kind][key] = Number(input.value);
  });
  container.dataset.predicates = JSON.stringify(predicates);
}
function routesFromBoard() {
  return [...routingCards.querySelectorAll('.route-card')].map(card => {
    let route = {};
    try { route = JSON.parse(card.dataset.route || '{}'); } catch (_) { /* use an empty draft */ }
    return {
      ...route,
      source: Number(card.querySelector('[data-route-key="source"]').value),
      destination: Number(card.querySelector('[data-route-key="destination"]').value),
      enabled: card.querySelector('[data-route-key="enabled"]').checked,
      priority: Number(card.querySelector('[data-route-key="priority"]').value),
      curve: card.querySelector('[data-route-key="curve"]').value,
      allow_cycle: card.querySelector('[data-route-key="allow_cycle"]').checked,
      predicates: (() => { try { const value = JSON.parse(card.querySelector('[data-route-key="predicates"]').dataset.predicates || '[]'); if (!Array.isArray(value)) throw new Error('must be an array'); return value; } catch (_) { return route.predicates || []; } })()
    };
  });
}
function syncRoutesJson() { routeDraft = routesFromBoard(); dirtyForm = true; routeDraftDirty = true; publishUiState(); }
routingCards.addEventListener('input', event => { const card = event.target.closest('.route-card'); if (card) refreshPredicateData(card); syncRoutesJson(); });
routingCards.addEventListener('change', syncRoutesJson);
routingCards.addEventListener('click', event => {
  const card = event.target.closest('.route-card');
  const predicateAdd = event.target.closest('[data-add-predicate]');
  const predicateRemove = event.target.closest('[data-remove-predicate]');
  if (card && (predicateAdd || predicateRemove)) {
    refreshPredicateData(card);
    const container = card.querySelector('[data-route-key="predicates"]');
    const predicates = JSON.parse(container.dataset.predicates || '[]');
    if (predicateRemove) predicates.splice(Number(predicateRemove.dataset.removePredicate), 1);
    if (predicateAdd?.dataset.addPredicate === 'NumberRange') predicates.push({ NumberRange: { minimum: 0, maximum: 127 } });
    if (predicateAdd?.dataset.addPredicate === 'ValueRange') predicates.push({ ValueRange: { minimum: 0, maximum: 16383 } });
    if (predicateAdd?.dataset.addPredicate === 'Realtime') predicates.push({ Realtime: 'Clock' });
    container.dataset.predicates = JSON.stringify(predicates);
    container.replaceChildren(Object.assign(document.createElement('legend'), { textContent: 'Conditions' }));
    renderPredicateBuilder(container, predicates);
    syncRoutesJson();
    return;
  }
  const remove = event.target.closest('[data-remove-route]');
  if (!remove) {
    const card = event.target.closest('.route-card');
    if (card) showInspector(`Route ${Number(card.dataset.routeIndex) + 1} selected${routeDraftDirty ? ' · unsaved draft' : ''}. Refresh to inspect authoritative state.`);
    return;
  }
  const routes = routesFromBoard(); routes.splice(Number(remove.dataset.removeRoute), 1); renderRoutingBoard(routes); syncRoutesJson();
});
document.querySelector('#routing-add').addEventListener('click', () => { const routes = routesFromBoard(); routes.push({ source: 0, destination: 0, enabled: true }); renderRoutingBoard(routes); syncRoutesJson(); });
themeButton.addEventListener('click', () => {
  document.body.classList.toggle('light');
  window.localStorage.setItem('mackes-theme', document.body.classList.contains('light') ? 'light' : 'dark');
  updateThemeLabel();
});
updateThemeLabel();
layoutToggle?.addEventListener('click', () => {
  if (layoutToggle.dataset.supported !== 'true') return;
  const generic = faceplateControls.classList.toggle('generic-layout');
  layoutToggle.setAttribute('aria-pressed', String(generic));
  layoutToggle.textContent = generic ? 'Use exact device layout' : 'Use generic layout';
  showInspector(`${generic ? 'Generic' : 'Exact device'} layout selected. Refresh to inspect authoritative state.`);
});
document.querySelectorAll('form').forEach(form => form.addEventListener('input', () => { dirtyForm = true; publishUiState(); }));
document.querySelectorAll('form').forEach(form => form.addEventListener('change', () => { dirtyForm = true; publishUiState(); }));
window.addEventListener('beforeunload', event => { if (dirtyForm) { event.preventDefault(); event.returnValue = ''; } });
window.addEventListener('offline', () => { reconnectBanner.hidden = false; health.textContent = 'Network unavailable'; });
window.addEventListener('online', () => { reconnectBanner.hidden = false; health.textContent = 'Reconnecting to daemon…'; load(activeView); });
document.addEventListener('visibilitychange', () => {
  if (document.hidden || browserSmoke) return;
  pollHealth();
  refreshActiveView();
});
async function load(view) {
  activeView = view;
  window.MackesStateStore?.publish({ view, generation: currentGeneration });
  const loadSequence = ++viewLoadSequence;
  const abortController = new AbortController();
  activeViewAbortController = abortController;
  title.textContent = labels[view];
  const focusedId = document.activeElement?.id;
  if (view === 'scenes') { operation.dataset.state = 'pending'; operation.textContent = 'Refreshing authoritative scenes, projects, and setlists…'; }
  try {
    const endpoint = `/api/v1/${view === 'system' || view === 'recovery' ? 'diagnostics' : view}`;
    /* Keep a slow broad inventory read from blocking device-specific views.
       The Novation read below has its own bounded timeout and can render the
       control grid even when the broad inventory is unavailable. */
    const response = await boundedFetch(endpoint, { signal: abortController.signal }, 12000);
    /* Keep workspace rendering alive when the broad view endpoint is briefly
       unavailable; device-specific reads below can still recover the page. */
    let body = {};
    try { body = await response.json(); } catch (_) { body = {}; }
    if (loadSequence !== viewLoadSequence) return;
    if (view === 'system') {
      try {
        const daemonResponse = await fetch('/api/v1/health');
        body.daemon_health = await daemonResponse.json();
      } catch (error) {
        body.daemon_health = { code: 'daemon_unavailable', message: String(error) };
      }
      renderSystemBoard(body);
    }
    if (Number.isInteger(body.generation)) {
      currentGeneration = body.generation;
      if (view === 'mappings') mappingGeneration = body.generation;
      if (view === 'assignment') assignmentGeneration = body.generation;
      publishUiState();
    }
    if (view === 'routes') {
      if (Number.isInteger(body.route_generation)) routeGeneration = body.route_generation;
      routeEndpointCatalog = Array.isArray(body.endpoint_catalog) ? body.endpoint_catalog : [];
      routeEndpointLossless = routeEndpointCatalog.every(endpoint => Number.isSafeInteger(endpoint?.id));
      if (!routeEndpointLossless) {
        showInspector('Named-port route editing is read-only: endpoint identifiers exceed JavaScript safe-integer precision. Use the raw configuration boundary until a lossless backend contract is enabled.');
      }
      const routes = routeListFromBody(body);
      if (!routeDraftDirty) { routeDraft = routes; renderRoutingBoard(routes); }
    }
    if (view === 'devices') {
      const [endpointResponse, novationResponse] = await Promise.all([
        boundedFetch('/api/v1/endpoints', { signal: abortController.signal }, 12000).catch(() => null), boundedFetch('/api/v1/novation', { signal: abortController.signal }, 12000).catch(() => null)
      ]);
      if (loadSequence !== viewLoadSequence) return;
      const endpointBody = endpointResponse?.ok ? await endpointResponse.json() : body;
      const novationBody = novationResponse?.ok ? await novationResponse.json() : {};
      const endpoints = Array.isArray(endpointBody?.endpoints) ? endpointBody.endpoints : [];
      if (novationBody.novation_capabilities && endpoints.length) {
        endpoints.push({ name: 'Novation Launch Control XL', transport: 'MIDI / SysEx', direction: 'input/output', state: novationBody.led?.phase || 'available', capabilities: Object.keys(novationBody.novation_capabilities) });
      }
      renderDeviceBoard(endpoints);
      /* PiPedal controls are only meaningful when the authoritative device
         inventory reports the connector. Refresh its typed snapshot alongside
         the Devices workspace so catalog/readback is not hidden behind a
         second manual workflow. */
      if (endpoints.some(device => /pipedal/i.test(JSON.stringify(device)))) {
        document.querySelector('#pipedal-refresh')?.click();
      }
      if (novationResponse?.ok) {
        renderFaceplate(novationBody);
        const controller = new AbortController();
        const timeout = window.setTimeout(() => controller.abort(), 12000);
        try {
          const mappingsResponse = await fetch('/api/v1/mappings', { signal: controller.signal });
          if (mappingsResponse.ok) {
            const mappingsBody = await mappingsResponse.json();
            if (loadSequence !== viewLoadSequence) return;
            if (Number.isInteger(mappingsBody.generation)) mappingGeneration = mappingsBody.generation;
            const mappings = Array.isArray(mappingsBody.mapping_registry)
              ? mappingsBody.mapping_registry
              : (Array.isArray(mappingsBody.active) ? mappingsBody.active : mappingsBody.control_mappings);
            renderFaceplate({ ...novationBody, mapping_registry: Array.isArray(mappings) ? mappings : [] });
          }
        } catch (error) {
          showInspector('Novation grid rendered; assignment state is unavailable until mappings refresh.');
        } finally { window.clearTimeout(timeout); }
      } else {
        renderFaceplate({});
        showInspector('Novation grid rendered; device state is unavailable. Reconnect and refresh.');
      }
      const capabilityResponse = await fetch('/api/v1/capabilities', { signal: abortController.signal });
      if (capabilityResponse.ok) renderCapabilityBoard(await capabilityResponse.json());
    }
    if (view === 'mappings') renderMappingLayers(body);
    if (view === 'scenes') renderSceneBoard(body);
    if (view === 'scenes' && operation.dataset.state === 'pending') { operation.dataset.state = 'complete'; operation.textContent = 'Authoritative scenes, projects, and setlists refreshed.'; }
    if (view !== 'monitor' || (!monitorPaused && !monitorCleared)) presentStatus(body);
    if (view === 'monitor' && monitorCleared) monitorCleared = false;
    if (response.ok) {
      consecutiveHealthFailures = 0;
      lastHealthSuccessAt = Date.now();
      health.textContent = `Backend ${body.health || 'online'} · /api/v1/health · checked now`;
      reconnectBanner.hidden = true;
    }
    if (focusedId) {
      const focused = document.getElementById(focusedId);
      if (focused && !focused.disabled) focused.focus({ preventScroll: true });
    }
  } catch (error) {
    if (error?.name === 'AbortError') return;
    state.textContent = `Data refresh delayed: ${error}`;
  }
}
async function pollHealth() {
  try {
    const response = await boundedFetch('/api/v1/health', {}, 5500);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const body = await response.json();
    consecutiveHealthFailures = 0;
    lastHealthSuccessAt = Date.now();
    health.textContent = `Backend ${body.health || 'online'} · /api/v1/health · checked now`;
    reconnectBanner.hidden = true;
  } catch (_) {
    consecutiveHealthFailures += 1;
    const status = window.MackesHealth.status({ failures: consecutiveHealthFailures, lastSuccessAt });
    health.textContent = status.text;
    reconnectBanner.hidden = !status.offline;
  }
}
async function refreshActiveView() {
  if (scheduledRefreshActive || document.visibilityState !== 'visible' || dirtyForm || routeDraftDirty) return;
  scheduledRefreshActive = true;
  try { await load(activeView); }
  finally { scheduledRefreshActive = false; }
}
async function pollEvents() {
  try {
    const response = await fetch(`/api/v1/events?after_sequence=${lastSequence}`);
    if (!response.ok) return;
    const body = await response.json();
    if (body.snapshot_required === true || body.error === 'event_gap') {
      lastSequence = Number.isInteger(body.last_sequence) ? body.last_sequence : 0;
      eventLog.length = 0;
      eventTimes.length = 0;
      updateMonitorStats();
      operation.textContent = 'Monitor resynchronized from authoritative state.';
      await load(activeView);
      return;
    }
    for (const event of body.events || []) {
      if (Number.isInteger(event.sequence) && event.sequence > lastSequence) {
        lastSequence = event.sequence;
        eventLog.push(event);
        eventTimes.push(Date.now());
        if (eventLog.length > 256) eventLog.shift();
      }
    }
    updateMonitorStats();
    if (activeView === 'monitor' && !monitorPaused && !monitorCleared) presentStatus({ events: visibleEvents() });
  } catch (_) { /* the regular view poll reports daemon availability */ }
}
let eventStream = null;
let eventStreamRetry = null;
function consumeStreamEvent(event) {
  try {
    const value = JSON.parse(event.data);
    if (event.type === 'resnapshot' || value.snapshot_required === true) {
      lastSequence = Number.isInteger(value.last_sequence) ? value.last_sequence : 0;
      eventLog.length = 0; eventTimes.length = 0; updateMonitorStats();
      operation.textContent = 'Monitor resynchronized from authoritative state.';
      load(activeView);
      return;
    }
    if (Number.isInteger(value.sequence) && value.sequence > lastSequence) {
      lastSequence = value.sequence; eventLog.push(value); eventTimes.push(Date.now());
      if (eventLog.length > 256) eventLog.shift();
      updateMonitorStats();
      if (activeView === 'monitor' && !monitorPaused && !monitorCleared) presentStatus({ events: visibleEvents() });
    }
  } catch (_) { /* malformed stream data is ignored; the next resnapshot repairs state */ }
}
function startEventStream() {
  if (!window.EventSource) return;
  if (eventStream) eventStream.close();
  eventStream = new EventSource(`/api/v1/events/stream?after_sequence=${lastSequence}`);
  eventStream.onmessage = consumeStreamEvent;
  eventStream.addEventListener('resnapshot', consumeStreamEvent);
  eventStream.onerror = () => {
    eventStream.close(); eventStream = null;
    if (!eventStreamRetry) eventStreamRetry = window.setTimeout(() => { eventStreamRetry = null; startEventStream(); }, 3000);
    pollEvents();
  };
}
document.querySelectorAll('[data-view]').forEach(button => button.addEventListener('click', () => navigate(button.dataset.view)));
const monitorControls = document.querySelector('#monitor-controls');
const deviceControl = document.querySelector('#device-control');
const assignmentControls = document.querySelector('#assignment-controls');
const assignmentCatalog = document.querySelector('#assignment-catalog');
const assignmentCatalogHeading = document.querySelector('#assignment-catalog-heading');
const mappingLayerControls = document.querySelector('#mapping-layer-controls');
const mappingLayerSelect = document.querySelector('#mapping-layer-select');
const mappingLayerApply = document.querySelector('#mapping-layer-apply');
const mappingLayerStatus = document.querySelector('#mapping-layer-status');
const assignmentChoiceLabel = document.querySelector('#assignment-choice-label');
const assignmentChoice = document.querySelector('#assignment-choice');
const assignmentSearch = document.querySelector('#assignment-search');
let assignmentEntries = [];
const faceplate = document.querySelector('#faceplate');
const faceplateControls = document.querySelector('#faceplate-controls');
const selectedPhysicalControl = document.querySelector('#selected-physical-control');
let selectedPhysicalControlId = '';
let mappingRegistry = [];
let selectedMappingId = '';
/* Mutation domains have independent optimistic-concurrency generations.
   Device refresh generations must not be reused for mapping/assignment writes. */
let mappingGeneration = 0;
let assignmentGeneration = 0;
let routeGeneration = 0;
let pipedalGeneration = 0;
let sceneCatalog = null;
let routeEndpointCatalog = [];
let routeEndpointLossless = true;
const routingControls = document.querySelector('#routing-controls');
const sceneControls = document.querySelector('#scene-controls');
const pipedalOperationChoice = document.querySelector('#pipedal-operation-choice');
const pipedalMappingChoice = document.querySelector('#pipedal-mapping-choice');
const pipedalInstanceId = document.querySelector('#pipedal-instance-id');
const pipedalValue = document.querySelector('#pipedal-value');
const pipedalRepairPlugin = document.querySelector('#pipedal-repair-plugin');
const pipedalRepairSymbol = document.querySelector('#pipedal-repair-symbol');
const pipedalConfirm = document.querySelector('#pipedal-confirm');
let pipedalMappings = [];
let pipedalCatalogControls = [];
function updatePipedalValueDomain() {
  const selected = pipedalMappings.find(entry => entry.physical_control_id === pipedalMappingChoice.value);
  const control = selected && pipedalCatalogControls.find(item => item && item.plugin_uri === selected.plugin_uri && item.symbol === selected.symbol);
  if (control && Number.isFinite(Number(control.min_value)) && Number.isFinite(Number(control.max_value))) {
    pipedalValue.min = String(control.min_value);
    pipedalValue.max = String(control.max_value);
    pipedalValue.title = `Range ${control.min_value} to ${control.max_value}`;
  } else {
    pipedalValue.removeAttribute('min');
    pipedalValue.removeAttribute('max');
    pipedalValue.removeAttribute('title');
  }
}
document.querySelectorAll('[data-view]').forEach(button => button.addEventListener('click', () => {
  deviceControl.hidden = button.dataset.view !== 'devices';
  assignmentControls.hidden = button.dataset.view !== 'mappings';
  routingControls.hidden = button.dataset.view !== 'routes';
  sceneControls.hidden = button.dataset.view !== 'scenes';
  document.querySelector('#system-board').hidden = button.dataset.view !== 'system';
}));
document.querySelector('#pause-monitor').addEventListener('click', event => {
  monitorPaused = !monitorPaused;
  publishUiState();
  event.target.textContent = monitorPaused ? 'Resume view' : 'Pause view';
});
document.querySelector('#clear-monitor').addEventListener('click', () => {
  monitorCleared = true;
  publishUiState();
  eventLog.length = 0;
  eventTimes.length = 0;
  updateMonitorStats();
  state.textContent = 'Monitor view cleared; daemon processing continues.';
});
document.querySelector('#download-monitor').addEventListener('click', () => {
  const blob = new Blob([JSON.stringify({ format: 'mackes-monitor-v1', events: visibleEvents() }, null, 2)], { type: 'application/json' });
  const link = document.createElement('a');
  link.href = URL.createObjectURL(blob);
  link.download = 'mackes-monitor.json';
  link.click();
  URL.revokeObjectURL(link.href);
  operation.textContent = `Downloaded ${visibleEvents().length} bounded monitor events.`;
});
document.querySelector('#monitor-endpoint').addEventListener('input', event => { monitorFilter.endpoint = event.target.value.trim(); if (!monitorPaused) presentStatus({ events: visibleEvents() }); });
document.querySelector('#monitor-channel').addEventListener('input', event => { monitorFilter.channel = event.target.value; if (!monitorPaused) presentStatus({ events: visibleEvents() }); });
document.querySelector('#monitor-class').addEventListener('change', event => { monitorFilter.kind = event.target.value; if (!monitorPaused) presentStatus({ events: visibleEvents() }); });
document.querySelectorAll('[data-view]').forEach(button => button.addEventListener('click', () => {
  activeView = button.dataset.view;
  window.MackesStateStore?.publish({ view: activeView, generation: currentGeneration });
  monitorControls.hidden = button.dataset.view !== 'monitor';
}));
async function runOperation(name, confirm = false, payload) {
  const request = { request_id: `web-${Date.now()}-${++operationSequence}`, operation: name, generation: currentGeneration, confirm };
  if (payload) request.payload = payload;
  operation.dataset.state = 'pending';
  operation.setAttribute('aria-busy', 'true');
  operation.textContent = `${name} pending…`;
  try {
    const response = await fetch('/api/v1/operations', {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(request)
    });
    const body = await response.json();
    if (Number.isInteger(body.route_generation)) routeGeneration = body.route_generation;
    if (Number.isInteger(body.generation)) { currentGeneration = body.generation; publishUiState(); }
    if (activeView === 'scenes') {
      sceneCatalog = body;
      const sceneChoice = document.querySelector('#scene-id');
      const active = body.active_scene || '';
      sceneChoice.replaceChildren(new Option('Choose an authoritative scene', ''));
      for (const entry of Array.isArray(body.scenes) ? body.scenes : []) {
        const id = typeof entry === 'string' ? entry : entry?.id;
        if (id) sceneChoice.add(new Option(id, id));
      }
      if (active) sceneChoice.value = active;
    }
    if (activeView === 'mappings') renderFaceplate(body);
    operation.textContent = response.ok
      ? `Accepted: ${body.operation_id}`
      : response.status === 409
        ? 'Conflict: state changed elsewhere; reloading authoritative state.'
        : `${body.code || 'Operation failed'} (${response.status})`;
    load('state');
  } catch (error) {
    operation.dataset.state = 'unknown';
    operation.textContent = `Operation outcome unknown; inspect state before retrying (${error})`;
  } finally {
    if (operation.dataset.state === 'pending') operation.dataset.state = 'complete';
    operation.setAttribute('aria-busy', 'false');
  }
}
function renderFaceplate(body) {
  const mappings = Array.isArray(body.mapping_registry)
    ? body.mapping_registry
    : (Array.isArray(body.control_mappings) ? body.control_mappings : []);
  mappingRegistry = mappings;
  if (assignmentCatalog) {
    const lifecycle = body?.lifecycle || body?.device?.lifecycle || 'unknown';
    const stableId = body?.stable_id || body?.device?.stable_id || 'unbound';
    const ledPhase = body?.led?.phase || 'unknown';
    const feedback = body?.led?.feedback_enabled === false ? 'disabled' : 'enabled/unknown';
    const currentLines = mappings.filter(item => item && item.enabled !== false).map(item => {
      const control = item.physical_control_id || item.physical_control || 'control';
      const destination = [item.destination_profile, item.destination_effect, item.destination_parameter].filter(Boolean).join(' / ') || item.id || 'unresolved';
      return `${control} → ${destination} · value unavailable (no authoritative readback)`;
    });
    assignmentCatalog.hidden = false;
    if (assignmentCatalogHeading) assignmentCatalogHeading.hidden = false;
    const refreshedAt = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    assignmentCatalog.textContent = `Novation ${lifecycle} · identity ${stableId} · LED ${ledPhase} · feedback ${feedback}\nCurrent assignments (${currentLines.length}) · refreshed ${refreshedAt}${currentLines.length ? `\n${currentLines.join('\n')}` : '\nNo active assignments reported by the authoritative mapping registry.'}`;
  }
  const rows = ['Novation Launch Control XL — text alternative', 'Knobs:'];
  const state = mapping => mapping ? `${mapping.enabled ? 'assigned' : 'disabled'}; led=${mapping.led || 'unspecified'}` : 'off';
  for (let row = 1; row <= 3; row += 1) {
    rows.push(Array.from({ length: 8 }, (_, col) => {
      const id = `knob-r${row}-c${col + 1}`;
      const mapping = mappings.find(item => (item.physical_control_id || item.physical_control) === id);
      return `${id}=${state(mapping)}`;
    }).join(' | '));
  }
  rows.push('Faders:');
  rows.push(Array.from({ length: 8 }, (_, index) => {
    const id = `fader-${index + 1}`;
    const mapping = mappings.find(item => (item.physical_control_id || item.physical_control) === id);
    return `${id}=${state(mapping)}`;
  }).join(' | '));
  rows.push('Buttons:');
  for (let row = 1; row <= 3; row += 1) {
    rows.push(Array.from({ length: 8 }, (_, col) => {
      const id = `button-r${row}-c${col + 1}`;
      const mapping = mappings.find(item => (item.physical_control_id || item.physical_control) === id);
      return `${id}=${state(mapping)}`;
    }).join(' | '));
  }
  faceplate.hidden = false;
  if (novationGridHeading) novationGridHeading.hidden = false;
  faceplate.textContent = rows.join('\n');
  faceplateControls.replaceChildren();
  const svg = (name, attributes = {}) => {
    const namespace = ['http', String.fromCharCode(58, 47, 47), 'www.w3.org/2000/svg'].join('');
    const node = document.createElementNS(namespace, name);
    for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, String(value));
    return node;
  };
  for (const [label, y] of [['KNOBS', 25], ['FADERS', 285], ['CHANNEL BUTTONS', 390], ['UTILITY', 515]]) {
    const text = svg('text', { x: 18, y, class: 'faceplate-row-label' }); text.textContent = label; faceplateControls.append(text);
  }
  const ids = [];
  for (const kind of ['knob']) for (let row = 1; row <= 3; row += 1)
    for (let col = 1; col <= 8; col += 1) ids.push(`${kind}-r${row}-c${col}`);
  for (let row = 1; row <= 2; row += 1)
    for (let col = 1; col <= 8; col += 1) ids.push(`button-r${row}-c${col}`);
  for (let index = 1; index <= 8; index += 1) ids.push(`fader-${index}`);
  for (let index = 1; index <= 8; index += 1) ids.push(`utility-${index}`);
  for (const id of ids) {
    const mapping = mappings.find(item => (item.physical_control_id || item.physical_control) === id);
    const kind = id.startsWith('knob') ? 'knob' : id.startsWith('fader') ? 'fader' : id.startsWith('utility') ? 'utility' : 'button';
    const numbers = id.match(/\d+/g).map(Number);
    const row = kind === 'fader' || kind === 'utility' ? 1 : numbers[0];
    const col = kind === 'fader' || kind === 'utility' ? numbers[0] : numbers[1];
    const x = 85 + (col - 1) * 115;
    const y = kind === 'knob' ? 70 + (row - 1) * 82 : kind === 'fader' ? 305 : kind === 'utility' ? 535 : 420 + (row - 1) * 55;
    const control = svg('g', {
      class: `faceplate-control ${mapping ? (mapping.enabled ? 'assigned' : 'disabled') : 'unassigned'}`,
      tabindex: 0, role: 'button', 'aria-label': `Select ${id}; ${mapping ? (mapping.enabled ? 'assigned' : 'disabled') : 'unassigned'}; current value unavailable until device readback`,
      'aria-pressed': selectedPhysicalControlId === id, 'data-physical-control-id': id
    });
    const shape = kind === 'knob'
      ? svg('circle', { cx: x, cy: y, r: 29, class: 'control-shape' })
      : svg('rect', { x: x - 40, y: y - (kind === 'fader' ? 22 : 18), width: 80, height: kind === 'fader' ? 44 : 36, rx: 7, class: 'control-shape' });
    const label = svg('text', { x, y: y + 5 });
    label.textContent = kind === 'knob' ? `K${row}.${col}` : kind === 'fader' ? `F${col}` : kind === 'utility' ? ['Device', 'Mute', 'Solo', 'Record', 'Up', 'Down', 'Left', 'Right'][col - 1] : `B${row}.${col}`;
    const assignmentLabel = svg('text', { x, y: y + (kind === 'knob' ? 48 : kind === 'fader' ? 38 : kind === 'utility' ? 18 : 30), class: 'assignment-label' });
    const assignment = mapping ? (mapping.destination_parameter || mapping.destination_effect || mapping.id || 'assigned') : '—';
    assignmentLabel.textContent = assignment.length > 14 ? `${assignment.slice(0, 13)}…` : assignment;
    const title = svg('title');
    title.textContent = mapping ? `${mapping.enabled ? 'Assigned' : 'Disabled'}: ${mapping.destination_parameter || mapping.id}; LED ${mapping.led || 'unspecified'}` : 'Unassigned';
    control.append(title, shape, label, assignmentLabel);
    const select = () => {
      selectedPhysicalControlId = id;
      selectedMappingId = mapping?.id || '';
      selectedPhysicalControl.textContent = `Selected physical control: ${id}`;
      if (mapping) {
        const destination = [mapping.destination_profile, mapping.destination_effect, mapping.destination_parameter].filter(Boolean).join(' / ') || 'destination not specified';
        const source = [mapping.source_endpoint, mapping.source_kind, Number.isInteger(mapping.source_channel) ? `ch ${mapping.source_channel}` : '', Number.isInteger(mapping.source_number) ? `#${mapping.source_number}` : ''].filter(Boolean).join(' · ') || 'source not specified';
        const behavior = mapping.behavior ? `${mapping.behavior.curve || 'linear'}${mapping.behavior.invert ? ' · inverted' : ''} · ${mapping.behavior.source_range?.join('–') || '0–127'} → ${mapping.behavior.destination_range?.join('–') || '0–127'}` : 'behavior not specified';
        showInspector(`Novation ${id} selected · ${mapping.enabled === false ? 'disabled' : 'assigned'}\nDestination: ${destination}\nSource: ${source}\nBehavior: ${behavior}\nLED: ${mapping.led || 'unspecified'} · mapping id: ${mapping.id || 'unnamed'}\nCurrent value: unavailable until authoritative device readback.`);
      } else {
        showInspector(`Novation ${id} selected · unassigned\nNo authoritative mapping exists for this control. Use the assignment workflow to create one; current value is unavailable until device readback.`);
      }
      workspaceInspector?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      const behavior = mapping?.behavior;
      document.querySelector('#mapping-behavior').hidden = !mapping?.id;
      document.querySelector('#mapping-toggle-enabled').textContent = mapping?.enabled ? 'Disable mapping' : 'Enable mapping';
      if (behavior) {
        document.querySelector('#mapping-source-min').value = behavior.source_range?.[0] ?? 0;
        document.querySelector('#mapping-source-max').value = behavior.source_range?.[1] ?? 127;
        document.querySelector('#mapping-destination-min').value = behavior.destination_range?.[0] ?? 0;
        document.querySelector('#mapping-destination-max').value = behavior.destination_range?.[1] ?? 127;
        document.querySelector('#mapping-curve').value = behavior.curve || 'linear';
        document.querySelector('#mapping-invert').checked = Boolean(behavior.invert);
      }
      faceplateControls.querySelectorAll('[data-physical-control-id]').forEach(item => item.setAttribute('aria-pressed', String(item === control)));
    };
    control.addEventListener('click', select);
    control.addEventListener('keydown', event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); select(); } });
    faceplateControls.append(control);
  }
  faceplateControls.hidden = false;
  if (!selectedPhysicalControlId) {
    const assigned = mappings.filter(item => item && item.enabled !== false);
    const assignmentSummary = `Novation assignments: ${assigned.length} active of ${ids.length}`;
    const examples = assigned.slice(0, 4).map(item => `${item.physical_control_id || item.physical_control}: ${item.destination_parameter || item.destination_effect || item.id}`).join(' · ');
    const refreshedAt = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    const lifecycle = body?.lifecycle || body?.device?.lifecycle || 'unknown';
    const stableId = body?.stable_id || body?.device?.stable_id || 'unbound';
    const ledPhase = body?.led?.phase || 'unknown';
    showInspector(`${assignmentSummary}. Novation ${lifecycle} · identity ${stableId} · LED ${ledPhase} · refreshed ${refreshedAt}. ${examples || 'No active assignments reported by the authoritative mapping registry.'} Select a control for details.`);
  }
  const supportsGenericLayout = body?.novation_capabilities?.generic_layout === true || body?.device?.supports_generic_layout === true;
  if (layoutToggle) {
    layoutToggle.hidden = !supportsGenericLayout;
    layoutToggle.dataset.supported = String(supportsGenericLayout);
  }
}
function renderMappingLayers(body) {
  const layers = body?.mapping_layers_v2;
  if (!mappingLayerControls || !mappingLayerSelect) return;
  mappingLayerControls.hidden = !layers;
  if (!layers) return;
  mappingLayerSelect.replaceChildren(new Option('Base layer', ''));
  for (const layer of layers.layers || []) mappingLayerSelect.add(new Option(layer.control_id, layer.control_id));
  mappingLayerSelect.value = layers.active_layer || '';
  if (mappingLayerStatus) mappingLayerStatus.textContent = `Authoritative layer: ${layers.active_layer || 'base'}`;
}
async function mappingMutation(operationName, payload, confirmation) {
  if (!selectedMappingId || (confirmation && !window.confirm(confirmation))) return;
  operation.dataset.state = 'pending';
  operation.setAttribute('aria-busy', 'true');
  operation.textContent = `Mapping ${operationName.toLowerCase()} pending…`;
  try {
    const response = await fetch('/api/v1/mappings', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ operation: operationName, generation: mappingGeneration, payload })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) { mappingGeneration = body.generation; publishUiState(); }
    operation.textContent = response.ok ? `Mapping ${operationName.toLowerCase()} applied.` : `Mapping ${operationName.toLowerCase()} failed (${response.status})`;
    if (response.ok) {
      showInspector(`Mapping ${selectedMappingId} ${operationName.toLowerCase()} acknowledged; awaiting observed state.`);
      await load('mappings');
      showInspector(`Mapping ${selectedMappingId} ${operationName.toLowerCase()} observed after authoritative refresh.`);
    } else showInspector(`Mapping ${selectedMappingId} rejected; draft remains available for correction.`);
  } catch (error) {
    operation.dataset.state = 'unknown';
    operation.textContent = `Mapping outcome unknown; inspect state before retrying (${error})`;
    showInspector(`Mapping ${selectedMappingId} outcome unknown; inspect authoritative state before retrying.`);
  } finally {
    if (operation.dataset.state === 'pending') operation.dataset.state = 'complete';
    operation.setAttribute('aria-busy', 'false');
  }
}
mappingLayerApply?.addEventListener('click', async () => {
  if (!mappingLayerSelect) return;
  mappingLayerStatus.textContent = 'Applying layer…';
  try {
    const response = await fetch('/api/v1/mappings', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ operation: 'SelectLayer', generation: mappingGeneration,
        payload: { kind: 'Layer', active_layer: mappingLayerSelect.value || null } })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) mappingGeneration = body.generation;
    mappingLayerStatus.textContent = response.ok ? 'Layer applied and persisted.' : `Layer rejected (${response.status}).`;
    if (response.ok) await load('mappings');
  } catch (error) { mappingLayerStatus.textContent = `Layer unavailable: ${error}`; }
});
document.querySelector('#mapping-toggle-enabled').addEventListener('click', () => {
  const mapping = mappingRegistry.find(item => item.id === selectedMappingId);
  if (mapping) mappingMutation('Enabled', { kind: 'Enabled', mapping_id: selectedMappingId, enabled: !mapping.enabled });
});
document.querySelector('#mapping-replace-destination').addEventListener('click', () => {
  const mapping = mappingRegistry.find(item => item.id === selectedMappingId);
  const profile = document.querySelector('#assignment-profile').value.trim();
  const effect = document.querySelector('#assignment-effect').value.trim();
  const parameter = document.querySelector('#assignment-parameter').value.trim();
  if (!mapping || !profile || !effect || !parameter) {
    operation.textContent = 'Select an assigned mapping and provide profile, effect, and parameter.';
    return;
  }
  const replacement = {
    id: mapping.id, controller_profile: mapping.controller_profile, physical_control_id: mapping.physical_control_id,
    source_endpoint: mapping.source_endpoint, source_kind: mapping.source_kind, source_channel: mapping.source_channel,
    destination_channel: mapping.destination_channel, source_number: mapping.source_number, destination_endpoint: mapping.destination_endpoint,
    destination_profile: profile, destination_effect: effect, destination_parameter: parameter,
    behavior: mapping.behavior, enabled: mapping.enabled, profile_version: mapping.profile_version
  };
  mappingMutation('Replace', { kind: 'Mapping', mapping: replacement }, 'Replace this mapping destination?');
});
document.querySelector('#mapping-delete').addEventListener('click', () =>
  mappingMutation('Delete', { kind: 'Delete', mapping_id: selectedMappingId }, 'Delete this mapping? This removes its active assignment.'));
document.querySelector('#mapping-preview').addEventListener('click', () => {
  if (!selectedMappingId) { operation.textContent = 'Select an assigned mapping before previewing changes.'; return; }
  const values = ['#mapping-source-min', '#mapping-source-max', '#mapping-destination-min', '#mapping-destination-max'].map(selector => Number(document.querySelector(selector).value));
  if (values.some(value => !Number.isInteger(value) || value < 0 || value > 65535) || values[0] > values[1] || values[2] > values[3]) { operation.textContent = 'Behavior ranges are invalid; preview was not created.'; return; }
  operation.textContent = 'Mapping preview ready; press Save behavior to apply explicitly.';
  showInspector(`Mapping ${selectedMappingId} previewed. No mutation sent; explicit Apply remains required.`);
});
document.querySelector('#mapping-save-behavior').addEventListener('click', async () => {
  const values = ['#mapping-source-min', '#mapping-source-max', '#mapping-destination-min', '#mapping-destination-max'].map(selector => Number(document.querySelector(selector).value));
  if (!selectedMappingId || values.some(value => !Number.isInteger(value) || value < 0 || value > 65535) || values[0] > values[1] || values[2] > values[3]) {
    operation.textContent = 'Behavior ranges are invalid or no assigned mapping is selected.';
    return;
  }
  operation.dataset.state = 'pending';
  operation.setAttribute('aria-busy', 'true');
  operation.textContent = 'Mapping behavior pending…';
  try {
    const response = await fetch('/api/v1/mappings', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ operation: 'Behavior', generation: mappingGeneration, payload: { kind: 'Behavior', mapping_id: selectedMappingId, behavior: { source_range: [values[0], values[1]], destination_range: [values[2], values[3]], invert: document.querySelector('#mapping-invert').checked, curve: document.querySelector('#mapping-curve').value } } })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) { mappingGeneration = body.generation; publishUiState(); }
    operation.textContent = response.ok ? 'Mapping behavior saved.' : `Behavior save failed (${response.status})`;
    if (response.ok) await load('mappings');
  } catch (error) {
    operation.dataset.state = 'unknown';
    operation.textContent = `Mapping behavior outcome unknown; inspect state before retrying (${error})`;
  } finally {
    if (operation.dataset.state === 'pending') operation.dataset.state = 'complete';
    operation.setAttribute('aria-busy', 'false');
  }
});
deviceControl.addEventListener('submit', event => {
  event.preventDefault();
  const payload = {
    profile_id: document.querySelector('#device-profile').value,
    control: document.querySelector('#device-control-name').value,
    destination: document.querySelector('#device-destination').value
  };
  if (!(payload.profile_id === 'lexicon.reflex' && payload.control === 'system-reset')) {
    payload.channel = Number(document.querySelector('#device-channel').value);
    payload.value = Number(document.querySelector('#device-value').value);
  }
  if (window.confirm('Send this device control to the selected destination?')) runOperation('device_control', true, payload);
});
document.querySelector('#rescan').addEventListener('click', () => runOperation('rescan'));
document.querySelector('#novation-refresh').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/novation');
    const body = await response.json();
    if (Number.isInteger(body.route_generation)) routeGeneration = body.route_generation;
    if (Number.isInteger(body.generation)) { currentGeneration = body.generation; publishUiState(); }
    presentStatus(body);
    /* Repaint the existing projection from this authoritative snapshot so a
       manual refresh updates assignment/lifecycle/readback context together. */
    renderFaceplate(body);
    const lifecycle = body.lifecycle || body.connection || body.status;
    const device = body.novation_device || body.device || body;
    const template = device.template ?? body.template;
    const pickup = device.pickup ?? body.pickup;
    const feedback = device.feedback_enabled ?? body.feedback_enabled;
    const diagnostics = document.querySelector('#novation-diagnostics');
    if (diagnostics) {
      const values = [
        lifecycle && `Lifecycle: ${typeof lifecycle === 'string' ? lifecycle : JSON.stringify(lifecycle)}`,
        template !== undefined && `Template: ${template}`,
        pickup !== undefined && `Pickup: ${pickup}`,
        feedback !== undefined && `LED feedback: ${feedback ? 'enabled' : 'disabled'}`
      ].filter(Boolean);
      diagnostics.textContent = values.length ? values.join(' · ') : 'Novation diagnostics unavailable in authoritative response.';
    }
    if (lifecycle || template !== undefined || pickup !== undefined || feedback !== undefined) {
      const details = [lifecycle && `state ${typeof lifecycle === 'string' ? lifecycle : JSON.stringify(lifecycle)}`, template !== undefined && `template ${template}`, pickup !== undefined && `pickup ${pickup}`, feedback !== undefined && `LED feedback ${feedback ? 'enabled' : 'disabled'}`].filter(Boolean).join(' · ');
      showInspector(`Novation device refreshed · ${details}.`);
    }
    operation.textContent = response.ok ? 'Novation device state refreshed.' : `Novation unavailable (${response.status})`;
  } catch (error) {
    const diagnostics = document.querySelector('#novation-diagnostics');
    if (diagnostics) diagnostics.textContent = 'Novation diagnostics unavailable; reconnect and refresh.';
    showInspector(`Novation refresh unavailable. Reconnect and refresh before editing.`); operation.textContent = `Novation unavailable: ${error}`;
  }
});
async function assignmentAction(action, payload) {
  operation.dataset.state = 'pending';
  operation.setAttribute('aria-busy', 'true');
  operation.textContent = `Assignment ${action.toLowerCase()} pending…`;
  try {
    let response = await fetch('/api/v1/assignment', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ generation: assignmentGeneration, action, ...(payload || {}) })
    });
    let body = await response.json();
    if (response.status === 409) {
      const conflict = String(body.error || body.message || '').toLowerCase();
      if (conflict.includes('ambiguous')) {
        operation.dataset.state = 'unknown';
        operation.textContent = 'Assignment unavailable: authoritative mapping is ambiguous; resolve duplicates before retrying.';
        return;
      }
      const fresh = await fetch('/api/v1/assignment');
      const snapshot = await fresh.json();
      if (Number.isInteger(snapshot.generation)) { assignmentGeneration = snapshot.generation; publishUiState(); }
      if (action === 'Commit' && (snapshot.session?.phase === 'Idle' || snapshot.session?.phase === 'Complete')) {
        const started = await fetch('/api/v1/assignment', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ generation: assignmentGeneration, action: 'Start' })
        });
        const startedBody = await started.json();
        if (Number.isInteger(startedBody.generation)) { assignmentGeneration = startedBody.generation; publishUiState(); }
      }
      response = await fetch('/api/v1/assignment', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ generation: assignmentGeneration, action, ...(payload || {}) })
      });
      body = await response.json();
    }
    if (Number.isInteger(body.generation)) { assignmentGeneration = body.generation; publishUiState(); }
    operation.textContent = response.ok ? `Assignment: ${body.session?.phase || 'updated'}` : `Assignment failed (${response.status})`;
    await loadAssignment();
  } catch (error) {
    operation.dataset.state = 'unknown';
    operation.textContent = `Assignment outcome unknown; inspect state before retrying (${error})`;
  } finally {
    if (operation.dataset.state === 'pending') operation.dataset.state = 'complete';
    operation.setAttribute('aria-busy', 'false');
  }
}
async function loadAssignment() {
  activeView = 'assignment';
  window.MackesStateStore?.publish({ view: activeView, generation: currentGeneration });
  try {
    const response = await fetch('/api/v1/assignment');
    const body = await response.json();
    if (Number.isInteger(body.generation)) { assignmentGeneration = body.generation; publishUiState(); }
    presentStatus(body);
    const catalog = body.session?.catalog || body.catalog || {};
    state.dataset.assignmentPhase = body.session?.phase || '';
    const choices = ['devices', 'presets', 'effects', 'types', 'parameters']
      .filter(key => Array.isArray(catalog[key]))
      .map(key => `${key}: ${catalog[key].length}`);
    assignmentCatalog.hidden = false;
    if (assignmentCatalogHeading) assignmentCatalogHeading.hidden = false;
    const activeMappings = Array.isArray(body.active) ? body.active : (Array.isArray(body.mapping_registry) ? body.mapping_registry : []);
    const assignmentLines = activeMappings.slice(0, 64).map(item => `${item.physical_control_id || item.physical_control || 'control'} → ${item.destination_profile || ''}${item.destination_effect ? ` / ${item.destination_effect}` : ''}${item.destination_parameter ? ` / ${item.destination_parameter}` : ''}${item.enabled === false ? ' [disabled]' : ''}`);
    assignmentCatalog.textContent = `Current assignments (${activeMappings.length}) — phase: ${body.session?.phase || 'unknown'}${assignmentLines.length ? `\n${assignmentLines.join('\n')}` : '\nNo current assignments reported.'}${choices.length ? `\nCatalog: ${choices.join(', ')}` : ''}`;
    const phaseCatalogKey = { ChooseDevice: 'devices', ChoosePreset: 'presets', ChooseEffect: 'effects', ChooseType: 'types', ChooseParameter: 'parameters' }[body.session?.phase];
    assignmentEntries = phaseCatalogKey && Array.isArray(catalog[phaseCatalogKey]) ? catalog[phaseCatalogKey].slice(0, 64) : [];
    renderAssignmentChoices();
    health.textContent = response.ok ? 'Daemon connected' : `Daemon unavailable (${response.status})`;
  } catch (error) {
    health.textContent = 'Daemon unavailable';
    state.textContent = String(error);
  }
}
function renderAssignmentChoices() {
  const query = assignmentSearch?.value.trim().toLowerCase() || '';
  const entries = assignmentEntries.filter(entry => !query || `${entry.label || ''} ${entry.id || ''}`.toLowerCase().includes(query));
  assignmentChoice.replaceChildren(new Option('Choose an authoritative catalog entry', ''));
  for (const entry of entries) assignmentChoice.add(new Option(`${entry.label || entry.id} (${entry.id})`, entry.id));
  assignmentChoiceLabel.hidden = assignmentEntries.length === 0;
  if (assignmentSearch) assignmentSearch.disabled = assignmentEntries.length === 0;
}
assignmentChoice.addEventListener('change', () => {
  const phase = assignmentChoice.value;
  const target = { ChooseDevice: '#assignment-profile', ChooseEffect: '#assignment-effect', ChooseParameter: '#assignment-parameter' }[state.dataset.assignmentPhase];
  if (target) document.querySelector(target).value = phase;
});
assignmentSearch?.addEventListener('input', renderAssignmentChoices);
document.querySelector('#assignment-snapshot').addEventListener('click', loadAssignment);
document.querySelectorAll('[data-assignment-action]').forEach(button => {
  button.addEventListener('click', () => {
    const action = button.dataset.assignmentAction;
    const physical = action === 'ControlCaptured' || action === 'Commit'
      ? selectedPhysicalControlId : null;
    if ((action === 'ControlCaptured' || action === 'Commit') && !physical) {
      operation.textContent = 'Select a physical control on the faceplate first.';
      return;
    }
    if (action === 'Commit') {
      const profile = document.querySelector('#assignment-profile').value.trim();
      const effect = document.querySelector('#assignment-effect').value.trim();
      const parameter = document.querySelector('#assignment-parameter').value.trim();
      if (!profile || !effect || !parameter) {
        operation.textContent = 'Choose a profile, effect, and parameter before committing.';
        return;
      }
      assignmentAction(action, {
        physical_control_id: physical,
        destination_profile: profile,
        destination_effect: effect,
        destination_parameter: parameter
      });
    } else {
      assignmentAction(action, physical === null ? undefined : { physical_control_id: physical });
    }
  });
});
document.querySelector('#mapping-undo').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/mappings', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ operation: 'Undo', generation: mappingGeneration })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) { mappingGeneration = body.generation; publishUiState(); }
    operation.textContent = response.ok ? 'Last mapping undone.' : `Mapping undo failed (${response.status})`;
    await load('mappings');
  } catch (error) { operation.textContent = `Mapping undo unavailable: ${error}`; }
});
document.querySelector('#routing-refresh').addEventListener('click', () => {
  if (routeDraftDirty && !window.confirm('Discard the unsaved route draft and reload authoritative routes?')) return;
  routeDraftDirty = false; load('routes');
});
document.querySelector('#routing-undo').addEventListener('click', async () => {
  operation.dataset.state = 'pending';
  operation.setAttribute('aria-busy', 'true');
  operation.textContent = 'Route undo pending…';
  try {
    const response = await fetch('/api/v1/routes', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ action: 'undo', route_generation: routeGeneration })
    });
    const body = await response.json();
    if (Number.isInteger(body.route_generation)) routeGeneration = body.route_generation;
    if (Number.isInteger(body.generation)) { currentGeneration = body.generation; publishUiState(); }
    operation.textContent = response.ok ? 'Last route change undone.' : `Route undo failed (${response.status})`;
    if (response.ok) routeDraftDirty = false;
    await load('routes');
  } catch (error) {
    operation.dataset.state = 'unknown';
    operation.textContent = `Route undo outcome unknown; inspect state before retrying (${error})`;
  } finally {
    if (operation.dataset.state === 'pending') operation.dataset.state = 'complete';
    operation.setAttribute('aria-busy', 'false');
  }
});
document.querySelector('#routing-preview').addEventListener('click', () => {
  try {
    const routes = routeDraft;
    const hopLimit = Number(document.querySelector('#route-hop-limit').value);
    if (!Array.isArray(routes)) throw new Error('routes must be an array');
    if (routes.length > 128) throw new Error('route count exceeds 128');
    if (!Number.isInteger(hopLimit) || hopLimit < 1 || hopLimit > 16) throw new Error('hop limit must be 1..16');
    operation.textContent = `Route preview valid: ${routes.length} route(s), hop limit ${hopLimit}; no changes applied.`;
  } catch (error) { operation.textContent = `Route preview rejected: ${error.message}`; }
});
async function rtpMutation(action) {
  const token = Number(document.querySelector('#rtp-token').value);
  const ssrc = Number(document.querySelector('#rtp-ssrc').value);
  if (!Number.isSafeInteger(token) || token < 0 || token > 0xffffffff || !Number.isSafeInteger(ssrc) || ssrc < 0 || ssrc > 0xffffffff) {
    operation.textContent = 'RTP token and SSRC must be bounded unsigned 32-bit values.'; return;
  }
  const response = await fetch('/api/v1/routes', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ action, token, ssrc }) });
  const body = await response.json();
  operation.textContent = response.ok ? `RTP peer ${action === 'rtp_establish' ? 'established' : 'ended'}.` : `RTP peer action failed (${response.status})`;
  if (body.generation) currentGeneration = body.generation;
}
document.querySelector('#rtp-establish').addEventListener('click', () => rtpMutation('rtp_establish'));
document.querySelector('#rtp-end').addEventListener('click', () => rtpMutation('rtp_end'));
document.querySelector('#rtp-discover').addEventListener('click', async () => {
  const output = document.querySelector('#rtp-discovery');
  try {
    const response = await fetch('/api/v1/endpoints');
    const body = await response.json();
    const endpoints = Array.isArray(body.endpoints) ? body.endpoints : [];
    output.textContent = response.ok ? `Discovered ${endpoints.length} daemon-authoritative endpoint(s): ${endpoints.map(item => item.name || item.id).join(', ') || 'none'}.` : `Discovery failed (${response.status}).`;
  } catch (error) { output.textContent = `Discovery unavailable: ${error}`; }
});
document.querySelector('#routing-apply').addEventListener('click', async () => {
  if (!routeEndpointLossless) {
    operation.textContent = 'Route apply blocked: endpoint identifiers require a lossless numeric contract.';
    return;
  }
  let routes;
  routes = routeDraft;
  if (!Array.isArray(routes)) { operation.textContent = 'Route draft is not ready; refresh the graphical board.'; return; }
  const hopLimit = Number(document.querySelector('#route-hop-limit').value);
  if (!Number.isInteger(hopLimit) || hopLimit < 1 || hopLimit > 16) {
    operation.textContent = 'Hop limit must be an integer from 1 to 16.';
    return;
  }
  try {
    operation.dataset.state = 'pending';
    operation.setAttribute('aria-busy', 'true');
    operation.textContent = 'Route apply pending…';
    const response = await fetch('/api/v1/routes', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ routes, hop_limit: hopLimit, route_generation: routeGeneration })
    });
    const body = await response.json();
    if (Number.isInteger(body.route_generation)) routeGeneration = body.route_generation;
    if (Number.isInteger(body.generation)) { currentGeneration = body.generation; publishUiState(); }
    operation.textContent = response.ok ? 'Routes applied.' : `Route apply failed (${response.status})`;
    if (response.ok) routeDraftDirty = false;
    await load('routes');
  } catch (error) {
    operation.dataset.state = 'unknown';
    operation.textContent = `Route apply outcome unknown; inspect state before retrying (${error})`;
  } finally {
    if (operation.dataset.state === 'pending') operation.dataset.state = 'complete';
    operation.setAttribute('aria-busy', 'false');
  }
});
document.querySelector('#pipedal-refresh').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/pipedal');
    const body = await response.json();
    if (Number.isInteger(body.generation)) pipedalGeneration = body.generation;
    if (Number.isInteger(body.generation)) { currentGeneration = body.generation; publishUiState(); }
    presentStatus(body);
    pipedalMappings = Array.isArray(body.mapping_resolution) ? body.mapping_resolution.filter(entry => entry && entry.physical_control_id) : [];
    pipedalCatalogControls = body.catalog && Array.isArray(body.catalog.controls) ? body.catalog.controls : [];
    const pipedalCatalogTargets = body.catalog && Array.isArray(body.catalog.targets) ? body.catalog.targets : [];
    pipedalMappingChoice.replaceChildren(new Option('Choose a persisted mapping', ''));
    for (const entry of pipedalMappings) {
      const label = `${entry.physical_control_id} → ${entry.symbol || 'parameter'} (${entry.status || 'unknown'})`;
      pipedalMappingChoice.add(new Option(label, entry.physical_control_id));
    }
    const catalogActions = Array.isArray(body.supported_operations)
      ? body.supported_operations.filter(value => ['setSnapshot', 'setControl', 'previewControl'].includes(value)) : [];
    const supportsBypass = Array.isArray(body.supported_operations) && body.supported_operations.includes('setPedalboardItemEnable');
    const supportsModUi = Array.isArray(body.supported_operations) && body.supported_operations.includes('setPedalboardItemUseModUi');
    const supportsTitle = Array.isArray(body.supported_operations) && body.supported_operations.includes('setPedalboardItemTitle');
    const supportsInputPreview = Array.isArray(body.supported_operations) && body.supported_operations.includes('previewInputVolume');
    const supportsOutputPreview = Array.isArray(body.supported_operations) && body.supported_operations.includes('previewOutputVolume');
    const supportsStatusMonitor = Array.isArray(body.supported_operations) && body.supported_operations.includes('getShowStatusMonitor');
    const supportsSaveAs = Array.isArray(body.supported_operations) && body.supported_operations.includes('saveCurrentPresetAs');
    const supportsPluginSaveAs = Array.isArray(body.supported_operations) && body.supported_operations.includes('savePluginPresetAs');
    const supportsLoadPreset = Array.isArray(body.supported_operations) && body.supported_operations.includes('loadPreset');
    const supported = catalogActions.length ? [['Snapshot', 'snapshot'], ['Apply', 'apply'], ['Repair', 'repair'], ['Undo', 'undo']].concat(supportsBypass ? [['Enable', 'enable'], ['Bypass', 'bypass']] : []).concat(supportsModUi ? [['Plugin UI', 'plugin_ui']] : []).concat(supportsTitle ? [['Rename', 'rename']] : []).concat(supportsInputPreview ? [['Preview input volume', 'preview_input']] : []).concat(supportsOutputPreview ? [['Preview output volume', 'preview_output']] : []).concat(supportsStatusMonitor ? [['Refresh status monitor', 'status_monitor']] : []).concat(supportsLoadPreset ? [['Load preset', 'load_preset']] : []).concat(supportsSaveAs ? [['Save current preset as', 'save_current_preset_as']] : []).concat(supportsPluginSaveAs ? [['Save plugin preset as', 'save_plugin_preset_as']] : []) : [];
    const selectedOperation = pipedalOperationChoice.value;
    pipedalOperationChoice.replaceChildren();
    for (const [label, value] of supported) pipedalOperationChoice.add(new Option(label, value));
    if (supported.some(([, value]) => value === selectedOperation)) pipedalOperationChoice.value = selectedOperation;
    if (!pipedalOperationChoice.value && supported.length) pipedalOperationChoice.value = supported[0][1];
    updatePipedalValueDomain();
    const count = Array.isArray(body.supported_operations) ? body.supported_operations.length : 0;
    const controlCount = pipedalCatalogControls.length;
    const domainCount = pipedalCatalogControls.filter(control => Number.isFinite(Number(control?.min_value)) && Number.isFinite(Number(control?.max_value))).length;
    const valueCount = pipedalCatalogControls.filter(control => Number.isFinite(Number(control?.value))).length;
    const nonFiniteValueCount = pipedalCatalogControls.filter(control => control && control.value != null && !Number.isFinite(Number(control.value))).length;
    const valueReadback = controlCount
      ? `${valueCount}/${controlCount} current control values${nonFiniteValueCount ? ` · ${nonFiniteValueCount} non-finite/unavailable` : ''}`
      : 'currentPedalboard control values unavailable in this snapshot';
    const controlRows = pipedalCatalogControls.slice(0, 3).map(control => {
      const label = control?.label || control?.symbol;
      const symbol = control?.symbol;
      if (!label) return null;
      const value = Number.isFinite(Number(control.value)) ? `value ${control.value}` : 'value unavailable';
      const domain = Number.isFinite(Number(control.min_value)) && Number.isFinite(Number(control.max_value)) ? `range ${control.min_value}..${control.max_value}` : 'range unavailable';
      return `${label}${symbol && symbol !== label ? ` [${symbol}]` : ''} (${value}; ${domain})`;
    }).filter(Boolean);
    const controlReadback = controlRows.length ? `controls: ${controlRows.join(' · ')}` : 'control rows unavailable';
    const lifecycleReadback = body.pipedal && typeof body.pipedal.phase === 'string' ? `daemon ${body.pipedal.phase}` : 'daemon lifecycle unavailable';
    const targetReadback = Array.isArray(pipedalCatalogTargets)
      ? `${pipedalCatalogTargets.length} pedalboard/plugin targets${pipedalCatalogTargets.slice(0, 3).map(target => { const instanceId = target?.instance_id ?? target?.instanceId; return target?.name && Number.isInteger(instanceId) ? `${target.name} (#${instanceId}; ${target.uri || 'plugin class unavailable'})${typeof target.bypassed === 'boolean' ? (target.bypassed ? ' bypassed' : ' active') : ' bypass unavailable'}` : null; }).filter(Boolean).length ? `: ${pipedalCatalogTargets.slice(0, 3).map(target => { const instanceId = target?.instance_id ?? target?.instanceId; return target?.name && Number.isInteger(instanceId) ? `${target.name} (#${instanceId}; ${target.uri || 'plugin class unavailable'})${typeof target.bypassed === 'boolean' ? (target.bypassed ? ' bypassed' : ' active') : ' bypass unavailable'}` : null; }).filter(Boolean).join(', ')}` : ''}`
      : 'currentPedalboard readback unavailable in this snapshot';
    const presetIndex = body.preset_index && typeof body.preset_index === 'object' ? body.preset_index : null;
    const bankIndex = body.bank_index && typeof body.bank_index === 'object' ? body.bank_index : null;
    const favorites = body.favorites && typeof body.favorites === 'object' && !Array.isArray(body.favorites) ? body.favorites : null;
    const favoriteEntries = favorites ? Object.entries(favorites).filter(([uri, value]) => uri && typeof value === 'boolean') : [];
    const favoritesReadback = favorites
      ? `${favoriteEntries.length} favorite plugin identities${favoriteEntries.filter(([, value]) => value).length ? ` (${favoriteEntries.filter(([, value]) => value).length} selected)` : ''}`
      : 'favorites readback unavailable in this snapshot';
    const systemMidiBindings = Array.isArray(body.system_midi_bindings) ? body.system_midi_bindings : null;
    const systemMidiReadback = systemMidiBindings
      ? `${systemMidiBindings.length} system MIDI bindings`
      : 'system MIDI readback unavailable in this snapshot';
    const governorReadback = typeof body.governor_settings === 'string' && body.governor_settings.trim()
      ? `governor ${body.governor_settings.slice(0, 64)}`
      : 'governor readback unavailable in this snapshot';
    const statusMonitorReadback = typeof body.show_status_monitor === 'boolean'
      ? `status monitor ${body.show_status_monitor ? 'shown' : 'hidden'}`
      : 'status monitor readback unavailable in this snapshot';
    const version = body.version && typeof body.version === 'object' ? body.version
      : (body.pipedal_version && typeof body.pipedal_version === 'object' ? body.pipedal_version : null);
    const versionReadback = version && typeof version.server_version === 'string' && version.server_version
      ? `PiPedal ${version.server_version}`
      : 'PiPedal version readback unavailable in this snapshot';
    const wifiDomains = body.wifi_regulatory_domains && typeof body.wifi_regulatory_domains === 'object' && !Array.isArray(body.wifi_regulatory_domains)
      ? `Wi-Fi regulatory domains ${Object.keys(body.wifi_regulatory_domains).length}`
      : 'Wi-Fi regulatory-domain readback unavailable in this snapshot';
    const presetReadback = presetIndex && Number.isInteger(presetIndex.selectedInstanceId) && Array.isArray(presetIndex.presets)
      ? `preset ${presetIndex.selectedInstanceId} (${presetIndex.presets.length} entries${presetIndex.presetChanged ? ', changed' : ''})` : null;
    const bankReadback = bankIndex && Number.isInteger(bankIndex.selectedBank) && Array.isArray(bankIndex.entries)
      ? `bank ${bankIndex.selectedBank} (${bankIndex.entries.length} entries)` : null;
    if (response.ok) {
      operation.textContent = `PiPedal catalog refreshed (${count} qualified operations).`;
      const presetBank = [presetReadback, bankReadback].filter(Boolean).join(' · ');
      showInspector(`PiPedal authoritative snapshot · ${lifecycleReadback} · ${versionReadback} · ${controlCount} catalog controls · ${domainCount} value domains · ${valueReadback} · ${controlReadback} · ${pipedalMappings.length} persisted mappings · ${targetReadback} · ${favoritesReadback} · ${systemMidiReadback} · ${governorReadback} · ${statusMonitorReadback} · ${wifiDomains} · ${presetBank || 'preset/bank readback unavailable in this snapshot'}. Readback is current for this snapshot; refresh after external changes.`);
    } else {
      operation.textContent = `PiPedal unavailable (${response.status})`;
      showInspector('PiPedal snapshot unavailable; displayed catalog and control domains may be stale. Refresh after reconnecting the daemon.');
    }
  } catch (error) {
    operation.textContent = `PiPedal unavailable: ${error}`;
    showInspector('PiPedal snapshot unavailable; displayed catalog and control domains are stale until a successful refresh.');
  }
});
pipedalMappingChoice.addEventListener('change', updatePipedalValueDomain);
document.querySelector('#pipedal-operation').addEventListener('click', async () => {
  const operationName = pipedalOperationChoice.value;
    const request = { operation: operationName, generation: pipedalGeneration, confirm: pipedalConfirm.checked };
  if (operationName === 'bypass' || operationName === 'enable') {
    const instanceId = Number(pipedalInstanceId.value);
    if (!Number.isSafeInteger(instanceId) || instanceId <= 0) { operation.textContent = 'PiPedal instance ID is required for bypass.'; return; }
    request.instance_id = instanceId;
    request.operation = 'apply';
    request.enabled = operationName === 'enable';
  }
  if (operationName === 'plugin_ui') {
    const instanceId = Number(pipedalInstanceId.value);
    if (!Number.isSafeInteger(instanceId) || instanceId <= 0) { operation.textContent = 'PiPedal instance ID is required for plugin UI mode.'; return; }
    request.operation = 'apply'; request.instance_id = instanceId; request.use_mod_ui = true;
  }
  if (operationName === 'rename') {
    const instanceId = Number(pipedalInstanceId.value);
    const title = window.prompt('PiPedal item title', '')?.trim();
    const colorKey = window.prompt('PiPedal icon color key', 'blue')?.trim();
    if (!Number.isSafeInteger(instanceId) || instanceId <= 0 || !title || !colorKey) { operation.textContent = 'Instance ID, title, and color key are required for rename.'; return; }
    request.operation = 'apply'; request.instance_id = instanceId; request.title = title; request.color_key = colorKey;
  }
  if (operationName === 'preview_input' || operationName === 'preview_output') {
    const volume = Number(window.prompt('Preview volume (dB)', '0'));
    if (!Number.isFinite(volume)) { operation.textContent = 'A finite preview volume is required.'; return; }
    request.operation = 'apply'; request.volume_db = volume; request.preview_input = operationName === 'preview_input';
  }
  if (operationName === 'status_monitor') {
    request.operation = 'apply';
    request.query_show_status_monitor = true;
  }
  if (operationName === 'load_preset') {
    const presetInstanceId = Number(window.prompt('Preset instance ID', '16'));
    if (!Number.isSafeInteger(presetInstanceId) || presetInstanceId <= 0) {
      operation.textContent = 'A positive preset instance ID is required.';
      return;
    }
    request.operation = 'apply';
    request.load_preset_instance_id = presetInstanceId;
  }
  if (operationName === 'save_current_preset_as') {
    const bankInstanceId = Number(window.prompt('Bank instance ID', '0'));
    const presetName = window.prompt('New preset name', '')?.trim();
    const saveAfterInstanceId = Number(window.prompt('Insert after preset instance ID', '-1'));
    if (!Number.isSafeInteger(bankInstanceId) || bankInstanceId < 0 || !presetName || !Number.isSafeInteger(saveAfterInstanceId) || saveAfterInstanceId < -1) {
      operation.textContent = 'Valid bank, preset name, and insertion IDs are required.';
      return;
    }
    request.operation = 'apply';
    request.bank_instance_id = bankInstanceId;
    request.preset_name = presetName;
    request.save_after_instance_id = saveAfterInstanceId;
  }
  if (operationName === 'save_plugin_preset_as') {
    const instanceId = Number(pipedalInstanceId.value);
    const presetName = window.prompt('New plugin preset name', '')?.trim();
    if (!Number.isSafeInteger(instanceId) || instanceId <= 0 || !presetName) {
      operation.textContent = 'A valid plugin instance ID and preset name are required.';
      return;
    }
    request.operation = 'apply';
    request.plugin_instance_id = instanceId;
    request.plugin_preset_name = presetName;
  }
  if (operationName === 'apply' || operationName === 'repair') {
    const selected = pipedalMappings.find(entry => entry.physical_control_id === pipedalMappingChoice.value);
    if (!selected) { operation.textContent = 'Choose a resolved persisted PiPedal mapping.'; return; }
    request.physical_control_id = selected.physical_control_id;
    if (operationName === 'repair') {
      const pluginUri = pipedalRepairPlugin.value.trim();
      const symbol = pipedalRepairSymbol.value.trim();
      if (!pluginUri || !symbol) { operation.textContent = 'Repair plugin URI and parameter symbol are required.'; return; }
      request.mapping = { physical_control_id: selected.physical_control_id, plugin_uri: pluginUri, symbol, scope: selected.scope || null };
    }
    request.instance_id = pipedalInstanceId.value.trim();
    request.value = Number(pipedalValue.value);
    if (operationName === 'apply' && !request.instance_id) { operation.textContent = 'PiPedal instance ID is required.'; return; }
    const min = pipedalValue.min === '' ? Number.NEGATIVE_INFINITY : Number(pipedalValue.min);
    const max = pipedalValue.max === '' ? Number.POSITIVE_INFINITY : Number(pipedalValue.max);
    if (operationName === 'apply' && (!Number.isFinite(request.value) || request.value < min || request.value > max)) {
      operation.textContent = 'PiPedal value must be a finite control-domain number.';
      return;
    }
  }
  if (operationName !== 'snapshot' && !pipedalConfirm.checked) { operation.textContent = 'Confirm the external PiPedal change first.'; return; }
  try {
    const response = await fetch('/api/v1/pipedal', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(request) });
    const body = await response.json();
    if (Number.isInteger(body.generation)) { pipedalGeneration = body.generation; publishUiState(); }
    operation.textContent = response.ok ? `PiPedal ${operationName} completed.` : `PiPedal failed (${response.status})`;
    presentStatus(body);
    if (response.ok && operationName === 'repair') document.querySelector('#pipedal-refresh').click();
  } catch (error) { operation.textContent = `PiPedal unavailable: ${error}`; }
});
document.querySelector('#panic').addEventListener('click', () => {
  if (window.confirm('Panic all outputs? This immediately stops active output.')) runOperation('panic', true);
});
async function navigateScene(direction) {
  try {
    const response = await fetch('/api/v1/scenes', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ direction })
    });
    const body = await response.json();
    operation.textContent = response.ok ? `${direction} scene requested.` : `${body.error || 'Scene action failed'} (${response.status})`;
    await load('scenes');
  } catch (error) { operation.textContent = `Scene action unavailable: ${error}`; }
}
document.querySelector('#previous-scene').addEventListener('click', () => navigateScene('previous'));
document.querySelector('#next-scene').addEventListener('click', () => navigateScene('next'));
document.querySelector('#scene-refresh').addEventListener('click', async () => {
  await load('scenes');
  operation.textContent = 'Scenes and setlists refreshed.';
});
document.querySelector('#preview-scene').addEventListener('click', async () => {
  const scene = document.querySelector('#scene-id').value.trim();
  if (!scene) { operation.textContent = 'Scene ID is required.'; return; }
  const entries = Array.isArray(sceneCatalog?.scenes) ? sceneCatalog.scenes : [];
  if (!entries.some(item => (typeof item === 'string' ? item : item?.id) === scene)) {
    operation.textContent = `Scene ${scene} is not present in the last authoritative catalog.`;
    return;
  }
  operation.dataset.state = 'pending';
  operation.setAttribute('aria-busy', 'true');
  operation.textContent = `Scene ${scene} preview pending…`;
  try {
    const response = await fetch('/api/v1/scenes', {
      method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({ preview_scene: scene })
    });
    const body = await response.json().catch(() => ({}));
    operation.textContent = response.ok
      ? `Scene ${scene} preview: ${body.preview || body.message || 'validated by daemon'}.`
      : `Scene preview failed (${response.status})`;
  } catch (error) {
    operation.dataset.state = 'unknown';
    operation.textContent = `Scene preview outcome unknown; inspect authoritative state before retrying (${error})`;
  } finally {
    if (operation.dataset.state === 'pending') operation.dataset.state = 'complete';
    operation.setAttribute('aria-busy', 'false');
  }
});
document.querySelector('#select-scene').addEventListener('click', async () => {
  const scene = document.querySelector('#scene-id').value.trim();
  if (!scene) { operation.textContent = 'Scene ID is required.'; return; }
  try {
    const response = await fetch('/api/v1/scenes', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ scene })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) { currentGeneration = body.generation; publishUiState(); }
    operation.textContent = response.ok ? `Scene ${scene} selected.` : `Scene selection failed (${response.status})`;
    await load('scenes');
  } catch (error) { operation.textContent = `Scene selection unavailable: ${error}`; }
});
document.querySelector('#execute-scene').addEventListener('click', async () => {
  const scene = document.querySelector('#scene-id').value.trim();
  if (!scene) { operation.textContent = 'Scene ID is required.'; return; }
  try {
    const response = await fetch('/api/v1/scenes', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ execute_scene: scene }) });
    const body = await response.json();
    const outcomes = Array.isArray(body.activation_outcomes) ? body.activation_outcomes : [];
    operation.textContent = response.ok ? `Scene ${scene} executed (${outcomes.length} action outcomes).` : `Scene execution failed (${response.status})`;
    presentStatus(body);
    await load('scenes');
  } catch (error) { operation.textContent = `Scene execution unavailable: ${error}`; }
});
document.querySelector('#download-diagnostics').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/diagnostics/bundle');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const blob = await response.blob();
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = 'mackes-diagnostics.json';
    link.click();
    URL.revokeObjectURL(link.href);
    operation.textContent = 'Diagnostic bundle downloaded.';
  } catch (error) { operation.textContent = `Diagnostics unavailable: ${error}`; }
});
document.querySelector('#download-configuration').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/configuration');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const blob = new Blob([JSON.stringify(await response.json(), null, 2)], { type: 'application/json' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = 'mackes-configuration.json';
    link.click();
    URL.revokeObjectURL(link.href);
    operation.textContent = 'Configuration exported.';
  } catch (error) { operation.textContent = `Configuration unavailable: ${error}`; }
});
document.querySelector('#download-raw-configuration').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/backups', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ action: 'export' }) });
    const body = await response.json();
    if (!response.ok || body.exported !== true) throw new Error(body.error || `HTTP ${response.status}`);
    const blob = new Blob([body.content], { type: 'application/json5' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = 'mackes-configuration.json5';
    link.click();
    URL.revokeObjectURL(link.href);
    operation.textContent = 'Raw configuration exported.';
  } catch (error) { operation.textContent = `Raw configuration unavailable: ${error}`; }
});
document.querySelector('#download-portable-configuration').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/backups', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ action: 'portable_export' }) });
    const body = await response.json();
    if (!response.ok || body.portable_exported !== true) throw new Error(body.error || `HTTP ${response.status}`);
    const blob = new Blob([body.content], { type: 'application/json' });
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = 'mackes-portable-config.json5';
    link.click();
    URL.revokeObjectURL(link.href);
    operation.textContent = 'Portable configuration exported.';
  } catch (error) { operation.textContent = `Portable export unavailable: ${error}`; }
});
const portableImportFile = document.querySelector('#portable-import-file');
document.querySelector('#import-portable-configuration').addEventListener('click', () => portableImportFile.click());
portableImportFile.addEventListener('change', async () => {
  const file = portableImportFile.files?.[0];
  portableImportFile.value = '';
  if (!file) return;
  if (file.size > 1024 * 1024) { operation.textContent = 'Portable import rejected: file exceeds 1 MiB.'; return; }
  const content = await file.text();
  if (!content) return;
  if (new TextEncoder().encode(content).length > 1024 * 1024) { operation.textContent = 'Portable import rejected: content exceeds 1 MiB.'; return; }
  if (!window.confirm('Replace the active configuration atomically and retain a backup?')) return;
  try {
    const response = await fetch('/api/v1/backups', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ action: 'portable_import', content, confirm: true }) });
    const body = await response.json();
    if (!response.ok || body.portable_imported !== true) throw new Error(body.error || `HTTP ${response.status}`);
    operation.textContent = 'Portable configuration imported.';
  } catch (error) { operation.textContent = `Portable import failed: ${error}`; }
});
document.querySelector('#validate-configuration').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/validation');
    const body = await response.json();
    presentStatus(body);
    operation.textContent = response.ok ? 'Configuration validation completed.' : `Validation failed (${response.status})`;
  } catch (error) { operation.textContent = `Validation unavailable: ${error}`; }
});
document.querySelector('#inspect-backups').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/backups');
    const body = await response.json();
    const backupChoice = document.querySelector('#backup-choice');
    backupChoice.replaceChildren(new Option('Choose an inventoried backup', ''));
    for (const backup of body.backups || []) {
      const name = typeof backup === 'string' ? backup : backup.name;
      if (name) backupChoice.add(new Option(name, name));
    }
    presentStatus(body);
    operation.textContent = response.ok ? 'Backup inventory refreshed.' : `Backup inventory failed (${response.status})`;
  } catch (error) { operation.textContent = `Backups unavailable: ${error}`; }
});
document.querySelector('#create-backup').addEventListener('click', async () => {
  if (!window.confirm('Create an immutable daemon-managed configuration backup?')) return;
  try {
    const response = await fetch('/api/v1/backups', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ action: 'create', confirm: true }) });
    const body = await response.json();
    operation.textContent = response.ok ? 'Configuration backup created.' : `Backup failed (${response.status})`;
    presentStatus(body);
  } catch (error) { operation.textContent = `Backup unavailable: ${error}`; }
});
document.querySelector('#restore-backup').addEventListener('click', async () => {
  const name = document.querySelector('#backup-choice').value;
  if (!name || !window.confirm(`Restore ${name} into the daemon configuration?`)) return;
  try {
    const response = await fetch('/api/v1/backups', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ action: 'restore', name, confirm: true }) });
    const body = await response.json();
    operation.textContent = response.ok ? 'Configuration backup restored.' : `Restore failed (${response.status})`;
    presentStatus(body);
  } catch (error) { operation.textContent = `Restore unavailable: ${error}`; }
});
document.querySelector('#open-guided-settings').addEventListener('click', () => { navigate('devices'); operation.textContent = 'Guided settings opened. Choose a graphical device or connection to continue.'; });
document.querySelector('#open-qualified-commands').addEventListener('click', () => { navigate('devices'); operation.textContent = 'Choose a qualified graphical device action. Unsupported protocol messages are not available in the normal workspace.'; });
window.addEventListener('popstate', () => navigate(viewFromLocation(), false));
if (!browserSmoke) {
  window.setInterval(pollHealth, 8000);
  window.setInterval(refreshActiveView, 10000);
  window.setInterval(() => { if (!eventStream) pollEvents(); }, 2000);
}
navigate(viewFromLocation(), false);
if (!browserSmoke) pollHealth();
if (!browserSmoke) startEventStream();
