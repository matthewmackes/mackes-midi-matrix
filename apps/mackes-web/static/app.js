const state = document.querySelector('#state');
const health = document.querySelector('#health');
const reconnectBanner = document.querySelector('#reconnect-banner');
const title = document.querySelector('#view-title');
const operation = document.querySelector('#operation');
const labels = { state: 'Live', mappings: 'Map Controls', assignment: 'Map Controls', routes: 'Routing', scenes: 'Scenes & Setlists', devices: 'Devices', system: 'System', monitor: 'Monitor' };
let monitorPaused = false;
let monitorCleared = false;
let currentGeneration = 0;
let operationSequence = 0;
let activeView = 'state';
let lastSequence = 0;
const eventLog = [];
const monitorFilter = { endpoint: '', channel: '', kind: '' };
const eventTimes = [];
let dirtyForm = false;
let routeDraftDirty = false;
const themeButton = document.querySelector('#theme');
const savedTheme = window.localStorage.getItem('mackes-theme');
const routingCards = document.querySelector('#routing-cards');
const routesJson = document.querySelector('#routes-json');
const deviceBoard = document.querySelector('#device-board');
const featureBoard = document.querySelector('#feature-board');
const featureFilterLabel = document.querySelector('#feature-filter-label');
const featureFilter = document.querySelector('#feature-filter');
let featureEntries = [];
const sceneBoard = document.querySelector('#scene-board');
const capabilityBoard = document.querySelector('#capability-board');
function viewFromLocation() { const view = window.location.pathname.split('/').filter(Boolean)[0] || 'state'; return Object.prototype.hasOwnProperty.call(labels, view) ? view : 'state'; }
function navigate(view, push = true) {
  if (activeView === 'routes' && view !== 'routes' && routeDraftDirty && !window.confirm('Leave Routing and discard the unsaved route draft?')) return;
  if (push) window.history.pushState({ view }, '', `/${view}`);
  document.querySelectorAll('[data-view]').forEach(button => {
    button.setAttribute('aria-current', button.dataset.view === view ? 'page' : 'false');
  });
  load(view);
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
  deviceBoard.replaceChildren();
  const endpointOptions = document.querySelector('#endpoint-options'); endpointOptions.replaceChildren();
  devices.forEach(device => { const option = document.createElement('option'); option.value = device.id || device.name || device.alias || ''; option.label = device.name || device.alias || option.value; if (option.value) endpointOptions.append(option); });
  if (!devices.length) { deviceBoard.hidden = true; return; }
  deviceBoard.hidden = false;
  devices.forEach((device, index) => {
    const card = document.createElement('article'); card.className = 'device-card';
    const name = device.name || device.alias || device.id || `Endpoint ${index + 1}`;
    const title = document.createElement('h3'); title.textContent = name; card.append(title);
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
  const catalog = [
    { match: /novation|launch control/i, name: 'Novation Launch Control XL', profile: 'novation.launch-control-xl', source: 'Qualified profile + Programmer Reference', features: ['24 knobs', '24 buttons', '8 faders', 'LED intent / delivery state', 'Template and reconnect diagnostics'] },
    { match: /eventide|micropitch/i, name: 'Eventide MicroPitch Delay', profile: 'eventide.micropitch', source: 'MicroPitch QRG + Eventide profile', features: [{ label: 'Expression', control: 'expression', cc: 4 }, { label: 'Tap trigger', control: 'tap', cc: 9 }, { label: 'Active / bypass', control: 'active', cc: 14 }, { label: 'FLEX', control: 'flex', cc: 15 }, { label: 'Mix', control: 'mix', cc: 20 }, { label: 'Pitch A', control: 'pitch-a', cc: 21 }, { label: 'Pitch B', control: 'pitch-b', cc: 22 }, { label: 'Depth', control: 'depth', cc: 23 }, { label: 'Rate sensitivity', control: 'rate-sensitivity', cc: 24 }, { label: 'Pitch mix', control: 'pitch-mix', cc: 25 }, { label: 'Tone', control: 'tone', cc: 26 }, { label: 'Delay A', control: 'delay-a', cc: 27 }, { label: 'Delay B', control: 'delay-b', cc: 28 }, { label: 'Modulation', control: 'modulation', cc: 29 }, { label: 'Feedback', control: 'feedback', cc: 30 }, { label: 'Output level', control: 'output-level', cc: 31 }] },
    { match: /lexicon|reflex/i, name: 'Lexicon Reflex', profile: 'lexicon.reflex', source: 'Reflex MIDI implementation + codec metadata', features: [{ label: 'Algorithm selector', control: 'algorithm-select' }, { label: 'Algorithm parameters', control: 'parameter' }, { label: 'Echo Rhythm', control: 'echo-rhythm' }, { label: 'MIDI patch 1–4', control: 'midi-patch' }, { label: 'Register read / recall', control: 'register-recall' }, { label: 'Register store (persistent)', control: 'register-store' }, { label: 'Setup / dump diagnostics', control: 'setup-dump' }, { label: 'Bypass / system task', control: 'bypass-task' }] },
    { match: /pipedal/i, name: 'PiPedal', profile: 'pipedal', source: 'Pinned server operation audit', features: ['Dynamic plugin controls', 'Snapshots and presets', 'MIDI bindings', 'Levels and pedalboard state', 'Unsupported operation reasons'] },
    { match: /midisport|m-audio/i, name: 'M-Audio MIDISPORT 4x4', profile: 'm-audio.midisport-4x4', source: 'Manufacturer capability evidence', features: ['4 MIDI inputs', '4 MIDI outputs', 'Per-port activity', 'Direction-aware aliases', 'Firmware and cable state separation'] },
    { match: /rtp|apple midi/i, name: 'RTP-MIDI session', profile: 'rtp-midi', source: 'AppleMIDI/RFC session requirements', features: ['Peer identity and allowlist', 'Invitation/handshake lifecycle', 'Sequence and reorder health', 'Reconnect backoff', 'Message delivery counters'] },
    { match: /midi through|generic midi/i, name: 'Generic MIDI transport', profile: 'generic-midi', source: 'Qualified MIDI engine and endpoint schemas', features: ['Direction-aware ports', 'Channel and message filters', 'CC/note/program/pressure/bend', 'SysEx and realtime predicates', 'Stable identity and reconnect repair'] }
  ];
  featureBoard.replaceChildren();
  const names = devices.map(device => String(device.name || device.alias || device.id || ''));
  featureEntries = catalog.map(item => ({ ...item, connected: names.some(name => item.match.test(name)) }));
  featureFilterLabel.hidden = !featureEntries.length;
  featureBoard.hidden = !featureEntries.length;
  renderFilteredFeatures();
}
function renderFilteredFeatures() {
  const query = featureFilter.value.trim().toLowerCase();
  const entries = featureEntries.filter(item => !query || `${item.name} ${item.source} ${item.features.map(feature => typeof feature === 'string' ? feature : `${feature.label} CC ${feature.cc}`).join(' ')}`.toLowerCase().includes(query));
  featureBoard.replaceChildren();
  if (!entries.length && featureEntries.length) { const empty = document.createElement('p'); empty.className = 'empty-state'; empty.textContent = 'No qualified product features match this filter.'; featureBoard.append(empty); return; }
  entries.forEach(item => {
    const card = document.createElement('article'); card.className = 'feature-card';
    const heading = document.createElement('h3'); heading.textContent = item.name; card.append(heading);
    const state = document.createElement('p'); state.className = item.connected ? 'feature-connected' : 'feature-disconnected'; state.textContent = item.connected ? 'Connected in live inventory' : 'Researched product · not connected'; card.append(state);
    const source = document.createElement('p'); source.className = 'feature-source'; source.textContent = `Evidence: ${item.source}`; card.append(source);
    const list = document.createElement('ul'); item.features.forEach(feature => {
      const li = document.createElement('li');
      const label = typeof feature === 'string' ? feature : `${feature.label}${feature.cc === undefined ? '' : ` (CC ${feature.cc})`}`;
      if (typeof feature === 'string') li.textContent = label;
      else { const select = document.createElement('button'); select.type = 'button'; select.textContent = label; select.addEventListener('click', () => { deviceControl.hidden = false; document.querySelector('#device-profile').value = item.profile; document.querySelector('#device-control-name').value = feature.control; document.querySelector('#device-value').value = feature.cc >= 14 && feature.cc <= 15 ? 127 : 0; document.querySelector('#device-control-name').focus(); operation.textContent = `${item.name} ${label} selected; verify destination and value before sending.`; }); li.append(select); }
      list.append(li);
    }); card.append(list);
    const edit = document.createElement('button'); edit.type = 'button'; edit.textContent = 'Open guarded control editor';
    edit.disabled = !item.connected;
    if (!item.connected) edit.textContent = 'Connect device to edit';
    edit.addEventListener('click', () => {
      if (!item.connected) return;
      deviceControl.hidden = false;
      if (item.profile === 'pipedal') { document.querySelector('#pipedal-refresh').focus(); operation.textContent = 'PiPedal workspace ready; refresh the authoritative plugin and operation catalog before choosing an operation.'; }
      else { document.querySelector('#device-profile').value = item.profile; document.querySelector('#device-control-name').focus(); operation.textContent = `${item.name} editor ready; choose a control, channel, value, and destination before sending.`; }
    });
    card.append(edit);
    featureBoard.append(card);
  });
}
featureFilter.addEventListener('input', renderFilteredFeatures);
function renderCapabilityBoard(body) {
  capabilityBoard.replaceChildren();
  const operations = body?.operations && typeof body.operations === 'object' ? Object.entries(body.operations) : [];
  capabilityBoard.hidden = !operations.length;
  if (!operations.length) return;
  const heading = document.createElement('h3'); heading.textContent = 'Platform capability coverage'; capabilityBoard.append(heading);
  const list = document.createElement('ul');
  operations.forEach(([name, status]) => { const item = document.createElement('li'); item.className = String(status).includes('partial') ? 'partial' : 'implemented'; item.textContent = `${name}: ${String(status).replaceAll('_', ' ')}`; list.append(item); });
  capabilityBoard.append(list);
  const unsupported = body?.unsupported?.remaining_mutations;
  if (unsupported) { const note = document.createElement('p'); note.className = 'capability-gap'; note.textContent = `Remaining mutation coverage: ${unsupported}`; capabilityBoard.append(note); }
}
function renderSceneBoard(body) {
  const scenes = Array.isArray(body?.scenes) ? body.scenes : [];
  const activeScene = body?.active_scene || body?.activeScene || '';
  sceneBoard.replaceChildren(); sceneBoard.hidden = !scenes.length;
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
    const select = document.createElement('button'); select.type = 'button'; select.textContent = 'Select scene'; select.dataset.sceneId = id; select.addEventListener('click', () => { document.querySelector('#scene-id').value = id; }); card.append(select);
    sceneBoard.append(card);
  });
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
      const input = document.createElement('input'); input.type = 'number'; input.min = '0'; input.max = '65535'; input.value = Number.isFinite(route[key]) ? route[key] : ''; input.dataset.routeKey = key; field.append(input); card.append(field);
    });
    const priority = document.createElement('label'); priority.textContent = 'Priority';
    const priorityInput = document.createElement('input'); priorityInput.type = 'number'; priorityInput.min = '-32768'; priorityInput.max = '32767'; priorityInput.value = Number.isInteger(route.priority) ? route.priority : 0; priorityInput.dataset.routeKey = 'priority'; priority.append(priorityInput); card.append(priority);
    const curve = document.createElement('label'); curve.textContent = 'Curve';
    const curveSelect = document.createElement('select'); curveSelect.dataset.routeKey = 'curve'; ['linear', 'inverse', 'square', 'exponential', 'logarithmic'].forEach(value => curveSelect.add(new Option(value, value))); curveSelect.value = route.curve || 'linear'; curve.append(curveSelect); card.append(curve);
    const enabled = document.createElement('label'); enabled.className = 'route-enabled';
    const checkbox = document.createElement('input'); checkbox.type = 'checkbox'; checkbox.checked = route.enabled !== false; checkbox.dataset.routeKey = 'enabled'; enabled.append(checkbox, document.createTextNode(' Connection enabled')); card.append(enabled);
    const cycle = document.createElement('label'); cycle.className = 'route-enabled';
    const cycleInput = document.createElement('input'); cycleInput.type = 'checkbox'; cycleInput.checked = route.allow_cycle === true; cycleInput.dataset.routeKey = 'allow_cycle'; cycle.append(cycleInput, document.createTextNode(' Allow cycles')); card.append(cycle);
    const predicates = document.createElement('label'); predicates.className = 'route-advanced'; predicates.textContent = 'Predicates (JSON)';
    const predicatesInput = document.createElement('textarea'); predicatesInput.rows = 2; predicatesInput.value = JSON.stringify(route.predicates || []); predicatesInput.dataset.routeKey = 'predicates'; predicates.append(predicatesInput); card.append(predicates);
    routingCards.append(card);
  });
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
      predicates: (() => { try { const value = JSON.parse(card.querySelector('[data-route-key="predicates"]').value || '[]'); if (!Array.isArray(value)) throw new Error('must be an array'); return value; } catch (_) { operation.textContent = 'Route draft has invalid predicate JSON; previous predicates retained until corrected.'; return route.predicates || []; } })()
    };
  });
}
function syncRoutesJson() { routesJson.value = JSON.stringify(routesFromBoard(), null, 2); dirtyForm = true; routeDraftDirty = true; }
routingCards.addEventListener('input', syncRoutesJson);
routingCards.addEventListener('change', syncRoutesJson);
routingCards.addEventListener('click', event => {
  const remove = event.target.closest('[data-remove-route]');
  if (!remove) return;
  const routes = routesFromBoard(); routes.splice(Number(remove.dataset.removeRoute), 1); renderRoutingBoard(routes); syncRoutesJson();
});
document.querySelector('#routing-add').addEventListener('click', () => { const routes = routesFromBoard(); routes.push({ source: 0, destination: 0, enabled: true }); renderRoutingBoard(routes); syncRoutesJson(); });
routesJson.addEventListener('input', () => { routeDraftDirty = true; dirtyForm = true; try { const routes = JSON.parse(routesJson.value || '[]'); if (Array.isArray(routes)) renderRoutingBoard(routes); } catch (_) { /* keep the text editor usable while JSON is incomplete */ } });
themeButton.addEventListener('click', () => {
  document.body.classList.toggle('light');
  window.localStorage.setItem('mackes-theme', document.body.classList.contains('light') ? 'light' : 'dark');
  updateThemeLabel();
});
updateThemeLabel();
document.querySelectorAll('form').forEach(form => form.addEventListener('input', () => { dirtyForm = true; }));
document.querySelectorAll('form').forEach(form => form.addEventListener('change', () => { dirtyForm = true; }));
window.addEventListener('beforeunload', event => { if (dirtyForm) { event.preventDefault(); event.returnValue = ''; } });
window.addEventListener('offline', () => { reconnectBanner.hidden = false; health.textContent = 'Network unavailable'; });
window.addEventListener('online', () => { reconnectBanner.hidden = false; health.textContent = 'Reconnecting to daemon…'; load(activeView); });
async function load(view) {
  activeView = view;
  title.textContent = labels[view];
  try {
    const endpoint = `/api/v1/${view === 'system' ? 'diagnostics' : view}`;
    const response = await fetch(endpoint);
    const body = await response.json();
    if (view === 'system') {
      try {
        const daemonResponse = await fetch('/api/v1/health');
        body.daemon_health = await daemonResponse.json();
      } catch (error) {
        body.daemon_health = { code: 'daemon_unavailable', message: String(error) };
      }
    }
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
    if (view === 'routes') {
      const routes = routeListFromBody(body);
      if (!routeDraftDirty) { renderRoutingBoard(routes); routesJson.value = JSON.stringify(routes, null, 2); }
    }
    if (view === 'devices') {
      const [endpointResponse, novationResponse] = await Promise.all([fetch('/api/v1/endpoints'), fetch('/api/v1/novation')]);
      const endpointBody = endpointResponse.ok ? await endpointResponse.json() : body;
      const novationBody = novationResponse.ok ? await novationResponse.json() : {};
      const endpoints = Array.isArray(endpointBody?.endpoints) ? endpointBody.endpoints : [];
      if (novationBody.novation_capabilities && endpoints.length) {
        endpoints.push({ name: 'Novation Launch Control XL', transport: 'MIDI / SysEx', direction: 'input/output', state: novationBody.led?.phase || 'available', capabilities: Object.keys(novationBody.novation_capabilities) });
      }
      renderDeviceBoard(endpoints);
      const capabilityResponse = await fetch('/api/v1/capabilities');
      if (capabilityResponse.ok) renderCapabilityBoard(await capabilityResponse.json());
    }
    if (view === 'scenes') renderSceneBoard(body);
    if (view !== 'monitor' || (!monitorPaused && !monitorCleared)) state.textContent = JSON.stringify(body, null, 2);
    if (view === 'monitor' && monitorCleared) monitorCleared = false;
    health.textContent = response.ok ? 'Daemon connected' : `Daemon unavailable (${response.status})`;
    reconnectBanner.hidden = response.ok;
  } catch (error) {
    health.textContent = 'Daemon unavailable';
    reconnectBanner.hidden = false;
    state.textContent = String(error);
  }
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
    if (activeView === 'monitor' && !monitorPaused && !monitorCleared) state.textContent = JSON.stringify(visibleEvents(), null, 2);
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
      if (activeView === 'monitor' && !monitorPaused && !monitorCleared) state.textContent = JSON.stringify(visibleEvents(), null, 2);
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
const assignmentChoiceLabel = document.querySelector('#assignment-choice-label');
const assignmentChoice = document.querySelector('#assignment-choice');
const faceplate = document.querySelector('#faceplate');
const faceplateControls = document.querySelector('#faceplate-controls');
const selectedPhysicalControl = document.querySelector('#selected-physical-control');
let selectedPhysicalControlId = '';
let mappingRegistry = [];
let selectedMappingId = '';
let sceneCatalog = null;
const routingControls = document.querySelector('#routing-controls');
const sceneControls = document.querySelector('#scene-controls');
const pipedalOperationChoice = document.querySelector('#pipedal-operation-choice');
const pipedalMappingChoice = document.querySelector('#pipedal-mapping-choice');
const pipedalInstanceId = document.querySelector('#pipedal-instance-id');
const pipedalValue = document.querySelector('#pipedal-value');
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
}));
document.querySelector('#pause-monitor').addEventListener('click', event => {
  monitorPaused = !monitorPaused;
  event.target.textContent = monitorPaused ? 'Resume view' : 'Pause view';
});
document.querySelector('#clear-monitor').addEventListener('click', () => {
  monitorCleared = true;
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
document.querySelector('#monitor-endpoint').addEventListener('input', event => { monitorFilter.endpoint = event.target.value.trim(); if (!monitorPaused) state.textContent = JSON.stringify(visibleEvents(), null, 2); });
document.querySelector('#monitor-channel').addEventListener('input', event => { monitorFilter.channel = event.target.value; if (!monitorPaused) state.textContent = JSON.stringify(visibleEvents(), null, 2); });
document.querySelector('#monitor-class').addEventListener('change', event => { monitorFilter.kind = event.target.value; if (!monitorPaused) state.textContent = JSON.stringify(visibleEvents(), null, 2); });
document.querySelectorAll('[data-view]').forEach(button => button.addEventListener('click', () => {
  activeView = button.dataset.view;
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
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
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
  faceplate.textContent = rows.join('\n');
  faceplateControls.replaceChildren();
  const ids = [];
  for (const kind of ['knob', 'button']) for (let row = 1; row <= 3; row += 1)
    for (let col = 1; col <= 8; col += 1) ids.push(`${kind}-r${row}-c${col}`);
  for (let index = 1; index <= 8; index += 1) ids.push(`fader-${index}`);
  for (const id of ids) {
    const mapping = mappings.find(item => (item.physical_control_id || item.physical_control) === id);
    const button = document.createElement('button');
    button.type = 'button';
    button.className = `faceplate-control ${id.startsWith('knob') ? 'knob' : id.startsWith('fader') ? 'fader-control' : 'button-control'}`;
    button.dataset.physicalControlId = id;
    button.textContent = id.replace(/^(knob|button)-/, '').replace('r', 'R').replace('-c', ' C');
    button.setAttribute('aria-label', `Select ${id}`);
    button.title = mapping ? `${mapping.enabled ? 'assigned' : 'disabled'}; LED ${mapping.led || 'unspecified'}` : 'unassigned';
    button.setAttribute('aria-pressed', String(selectedPhysicalControlId === id));
    button.addEventListener('click', () => {
      selectedPhysicalControlId = id;
      selectedMappingId = mapping?.id || '';
      selectedPhysicalControl.textContent = `Selected physical control: ${id}`;
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
      faceplateControls.querySelectorAll('button').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
    });
    faceplateControls.append(button);
  }
  faceplateControls.hidden = false;
}
async function mappingMutation(operationName, payload, confirmation) {
  if (!selectedMappingId || (confirmation && !window.confirm(confirmation))) return;
  operation.dataset.state = 'pending';
  operation.setAttribute('aria-busy', 'true');
  operation.textContent = `Mapping ${operationName.toLowerCase()} pending…`;
  try {
    const response = await fetch('/api/v1/mappings', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ operation: operationName, generation: currentGeneration, payload })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
    operation.textContent = response.ok ? `Mapping ${operationName.toLowerCase()} applied.` : `Mapping ${operationName.toLowerCase()} failed (${response.status})`;
    if (response.ok) await load('mappings');
  } catch (error) {
    operation.dataset.state = 'unknown';
    operation.textContent = `Mapping outcome unknown; inspect state before retrying (${error})`;
  } finally {
    if (operation.dataset.state === 'pending') operation.dataset.state = 'complete';
    operation.setAttribute('aria-busy', 'false');
  }
}
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
      body: JSON.stringify({ operation: 'Behavior', generation: currentGeneration, payload: { kind: 'Behavior', mapping_id: selectedMappingId, behavior: { source_range: [values[0], values[1]], destination_range: [values[2], values[3]], invert: document.querySelector('#mapping-invert').checked, curve: document.querySelector('#mapping-curve').value } } })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
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
    channel: Number(document.querySelector('#device-channel').value),
    value: Number(document.querySelector('#device-value').value),
    destination: document.querySelector('#device-destination').value
  };
  if (window.confirm('Send this device control to the selected destination?')) runOperation('device_control', true, payload);
});
document.querySelector('#rescan').addEventListener('click', () => runOperation('rescan'));
document.querySelector('#novation-refresh').addEventListener('click', async () => {
  try {
    const response = await fetch('/api/v1/novation');
    const body = await response.json();
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
    state.textContent = JSON.stringify(body, null, 2);
    operation.textContent = response.ok ? 'Novation device state refreshed.' : `Novation unavailable (${response.status})`;
  } catch (error) { operation.textContent = `Novation unavailable: ${error}`; }
});
async function assignmentAction(action, payload) {
  operation.dataset.state = 'pending';
  operation.setAttribute('aria-busy', 'true');
  operation.textContent = `Assignment ${action.toLowerCase()} pending…`;
  try {
    const response = await fetch('/api/v1/assignment', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ generation: currentGeneration, action, ...(payload || {}) })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
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
  try {
    const response = await fetch('/api/v1/assignment');
    const body = await response.json();
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
    state.textContent = JSON.stringify(body, null, 2);
    const catalog = body.session?.catalog || body.catalog || {};
    state.dataset.assignmentPhase = body.session?.phase || '';
    const choices = ['devices', 'presets', 'effects', 'types', 'parameters']
      .filter(key => Array.isArray(catalog[key]))
      .map(key => `${key}: ${catalog[key].length}`);
    assignmentCatalog.hidden = false;
    assignmentCatalog.textContent = `Authoritative assignment catalog — phase: ${body.session?.phase || 'unknown'}${choices.length ? `; ${choices.join(', ')}` : ''}`;
    const phaseCatalogKey = { ChooseDevice: 'devices', ChoosePreset: 'presets', ChooseEffect: 'effects', ChooseType: 'types', ChooseParameter: 'parameters' }[body.session?.phase];
    const entries = phaseCatalogKey && Array.isArray(catalog[phaseCatalogKey]) ? catalog[phaseCatalogKey].slice(0, 64) : [];
    assignmentChoice.replaceChildren(new Option('Choose an authoritative catalog entry', ''));
    for (const entry of entries) assignmentChoice.add(new Option(`${entry.label || entry.id} (${entry.id})`, entry.id));
    assignmentChoiceLabel.hidden = entries.length === 0;
    health.textContent = response.ok ? 'Daemon connected' : `Daemon unavailable (${response.status})`;
  } catch (error) {
    health.textContent = 'Daemon unavailable';
    state.textContent = String(error);
  }
}
assignmentChoice.addEventListener('change', () => {
  const phase = assignmentChoice.value;
  const target = { ChooseDevice: '#assignment-profile', ChooseEffect: '#assignment-effect', ChooseParameter: '#assignment-parameter' }[state.dataset.assignmentPhase];
  if (target) document.querySelector(target).value = phase;
});
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
      body: JSON.stringify({ operation: 'Undo', generation: currentGeneration })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
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
      body: JSON.stringify({ action: 'undo', route_generation: currentGeneration })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
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
    const routes = JSON.parse(document.querySelector('#routes-json').value || '[]');
    const hopLimit = Number(document.querySelector('#route-hop-limit').value);
    if (!Array.isArray(routes)) throw new Error('routes must be an array');
    if (routes.length > 128) throw new Error('route count exceeds 128');
    if (!Number.isInteger(hopLimit) || hopLimit < 1 || hopLimit > 16) throw new Error('hop limit must be 1..16');
    operation.textContent = `Route preview valid: ${routes.length} route(s), hop limit ${hopLimit}; no changes applied.`;
  } catch (error) { operation.textContent = `Route preview rejected: ${error.message}`; }
});
document.querySelector('#routing-apply').addEventListener('click', async () => {
  let routes;
  try {
    routes = JSON.parse(document.querySelector('#routes-json').value || '[]');
    if (!Array.isArray(routes)) throw new Error('routes must be an array');
  } catch (error) {
    operation.textContent = `Invalid route JSON: ${error.message}`;
    return;
  }
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
      body: JSON.stringify({ routes, hop_limit: hopLimit, route_generation: currentGeneration })
    });
    const body = await response.json();
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
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
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
    state.textContent = JSON.stringify(body, null, 2);
    pipedalMappings = Array.isArray(body.mapping_resolution) ? body.mapping_resolution.filter(entry => entry && entry.physical_control_id) : [];
    pipedalCatalogControls = body.catalog && Array.isArray(body.catalog.controls) ? body.catalog.controls : [];
    pipedalMappingChoice.replaceChildren(new Option('Choose a persisted mapping', ''));
    for (const entry of pipedalMappings) {
      const label = `${entry.physical_control_id} → ${entry.symbol || 'parameter'} (${entry.status || 'unknown'})`;
      pipedalMappingChoice.add(new Option(label, entry.physical_control_id));
    }
    const catalogActions = Array.isArray(body.supported_operations)
      ? body.supported_operations.filter(value => ['setSnapshot', 'setControl', 'previewControl'].includes(value)) : [];
    const supported = catalogActions.length ? ['Snapshot', 'Apply', 'Undo'] : [];
    const selectedOperation = pipedalOperationChoice.value;
    pipedalOperationChoice.replaceChildren();
    for (const value of supported) pipedalOperationChoice.add(new Option(value, value));
    if (supported.includes(selectedOperation)) pipedalOperationChoice.value = selectedOperation;
    if (!pipedalOperationChoice.value && supported.length) pipedalOperationChoice.value = supported[0];
    updatePipedalValueDomain();
    const count = Array.isArray(body.supported_operations) ? body.supported_operations.length : 0;
    operation.textContent = response.ok
      ? `PiPedal catalog refreshed (${count} qualified operations).`
      : `PiPedal unavailable (${response.status})`;
  } catch (error) { operation.textContent = `PiPedal unavailable: ${error}`; }
});
pipedalMappingChoice.addEventListener('change', updatePipedalValueDomain);
document.querySelector('#pipedal-operation').addEventListener('click', async () => {
  const operationName = pipedalOperationChoice.value;
  const request = { operation: operationName, generation: currentGeneration, confirm: pipedalConfirm.checked };
  if (operationName === 'Apply') {
    const selected = pipedalMappings.find(entry => entry.physical_control_id === pipedalMappingChoice.value);
    if (!selected) { operation.textContent = 'Choose a resolved persisted PiPedal mapping.'; return; }
    request.physical_control_id = selected.physical_control_id;
    request.instance_id = pipedalInstanceId.value.trim();
    request.value = Number(pipedalValue.value);
    if (!request.instance_id) { operation.textContent = 'PiPedal instance ID is required.'; return; }
    const min = pipedalValue.min === '' ? Number.NEGATIVE_INFINITY : Number(pipedalValue.min);
    const max = pipedalValue.max === '' ? Number.POSITIVE_INFINITY : Number(pipedalValue.max);
    if (!Number.isFinite(request.value) || request.value < min || request.value > max) {
      operation.textContent = 'PiPedal value must be a finite control-domain number.';
      return;
    }
  }
  if (operationName !== 'Snapshot' && !pipedalConfirm.checked) { operation.textContent = 'Confirm the external PiPedal change first.'; return; }
  try {
    const response = await fetch('/api/v1/pipedal', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(request) });
    const body = await response.json();
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
    operation.textContent = response.ok ? `PiPedal ${operationName} completed.` : `PiPedal failed (${response.status})`;
    state.textContent = JSON.stringify(body, null, 2);
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
document.querySelector('#preview-scene').addEventListener('click', () => {
  const scene = document.querySelector('#scene-id').value.trim();
  if (!scene) { operation.textContent = 'Scene ID is required.'; return; }
  const entries = Array.isArray(sceneCatalog?.scenes) ? sceneCatalog.scenes : [];
  const found = entries.some(item => (typeof item === 'string' ? item : item?.id) === scene);
  operation.textContent = found
    ? `Scene ${scene} is available; preview performed without recall.`
    : `Scene ${scene} is not present in the last authoritative catalog.`;
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
    if (Number.isInteger(body.generation)) currentGeneration = body.generation;
    operation.textContent = response.ok ? `Scene ${scene} selected.` : `Scene selection failed (${response.status})`;
    await load('scenes');
  } catch (error) { operation.textContent = `Scene selection unavailable: ${error}`; }
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
    state.textContent = JSON.stringify(body, null, 2);
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
    state.textContent = JSON.stringify(body, null, 2);
    operation.textContent = response.ok ? 'Backup inventory refreshed.' : `Backup inventory failed (${response.status})`;
  } catch (error) { operation.textContent = `Backups unavailable: ${error}`; }
});
document.querySelector('#create-backup').addEventListener('click', async () => {
  if (!window.confirm('Create an immutable daemon-managed configuration backup?')) return;
  try {
    const response = await fetch('/api/v1/backups', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ action: 'create', confirm: true }) });
    const body = await response.json();
    operation.textContent = response.ok ? 'Configuration backup created.' : `Backup failed (${response.status})`;
    state.textContent = JSON.stringify(body, null, 2);
  } catch (error) { operation.textContent = `Backup unavailable: ${error}`; }
});
document.querySelector('#restore-backup').addEventListener('click', async () => {
  const name = document.querySelector('#backup-choice').value;
  if (!name || !window.confirm(`Restore ${name} into the daemon configuration?`)) return;
  try {
    const response = await fetch('/api/v1/backups', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ action: 'restore', name, confirm: true }) });
    const body = await response.json();
    operation.textContent = response.ok ? 'Configuration backup restored.' : `Restore failed (${response.status})`;
    state.textContent = JSON.stringify(body, null, 2);
  } catch (error) { operation.textContent = `Restore unavailable: ${error}`; }
});
document.querySelector('#send-sysex').addEventListener('click', async () => {
  const destination = document.querySelector('#sysex-destination').value.trim();
  const text = document.querySelector('#sysex-bytes').value.trim();
  if (!destination || !text) { operation.textContent = 'SysEx rejected: destination and data are required.'; return; }
  if (!document.querySelector('#sysex-confirm').checked) { operation.textContent = 'Confirm the hardware-affecting SysEx action first.'; return; }
  const bytes = text.trim().split(/\s+/).map(Number);
  if (bytes.length > 1024 || !bytes.length || bytes.some(value => !Number.isInteger(value) || value < 0 || value > 127)) {
    operation.textContent = 'SysEx rejected: use 1–1024 decimal data bytes in the 0–127 range.';
    return;
  }
  try {
    const response = await fetch('/api/v1/sysex', { method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify({ destination, bytes, confirm: true }) });
    const body = await response.json();
    operation.textContent = response.ok ? 'SysEx accepted by daemon.' : `SysEx failed (${response.status})`;
    state.textContent = JSON.stringify(body, null, 2);
    if (response.ok) {
      document.querySelector('#sysex-bytes').value = '';
      document.querySelector('#sysex-confirm').checked = false;
    }
  } catch (error) { operation.textContent = `SysEx unavailable: ${error}`; }
});
window.addEventListener('popstate', () => navigate(viewFromLocation(), false));
window.setInterval(() => load(activeView), 2000);
window.setInterval(() => { if (!eventStream) pollEvents(); }, 2000);
navigate(viewFromLocation(), false);
startEventStream();
