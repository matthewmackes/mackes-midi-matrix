(() => {
  const health = document.querySelector('#studio-health');
  const announcement = document.querySelector('#studio-announcement');
  const theme = document.querySelector('#studio-theme');
  const state = window.MackesStudioState;
  const friendly = value => String(value || '').replace(/[._:-]+/g, ' ').replace(/\b\w/g, character => character.toUpperCase());
  const setVisualLevel = (control, value) => {
    const number = Number(value);
    if (!Number.isFinite(number)) return;
    const level = Math.max(0, Math.min(1, number > 127 ? number / 16383 : number / 127));
    control.style.setProperty('--control-level', String(level));
    control.style.setProperty('--control-angle', `${-135 + (level * 270)}deg`);
  };
  const view = window.location.pathname.split('/').filter(Boolean)[1] || 'controller';
  const viewCopy = {
    controller: ['LIVE CONTROLLER', 'Launch Control XL', 'Choose a knob, fader, or button to assign a musical function.'],
    devices: ['DEVICES', 'Connected devices', 'Browse authoritative device catalogs and connection health.'],
    routing: ['ROUTING', 'Signal routing', 'Inspect how controls and destinations are connected.'],
    scenes: ['SCENES', 'Performance scenes', 'Recall a prepared control layout without losing assignments.'],
    system: ['SYSTEM', 'Studio system', 'Review synchronization, diagnostics, and recovery state.']
  }[view] || null;
  if (viewCopy) {
    document.body.dataset.studioView = view;
    const eyebrow = document.querySelector('.page-heading .eyebrow');
    const heading = document.querySelector('.page-heading h1');
    const lede = document.querySelector('.page-heading .lede');
    if (eyebrow) eyebrow.textContent = viewCopy[0];
    if (heading) heading.textContent = viewCopy[1];
    if (lede) lede.textContent = viewCopy[2];
    document.querySelectorAll('.studio-nav a').forEach(link => {
      const active = new URL(link.href, window.location.origin).pathname === window.location.pathname;
      link.classList.toggle('nav-active', active);
      if (active) link.setAttribute('aria-current', 'page'); else link.removeAttribute('aria-current');
    });
    state.publish({ view });
    if (view !== 'controller' && announcement) announcement.textContent = `${viewCopy[1]} view is ready for Luna's next workflow slice.`;
  }
  const setTheme = value => { document.body.classList.toggle('light-theme', value === 'light'); if (theme) theme.textContent = value === 'light' ? 'Dark theme' : 'Light theme'; window.localStorage.setItem('mackes-studio-theme', value); };
  setTheme(window.localStorage.getItem('mackes-studio-theme') === 'light' ? 'light' : 'dark');
  theme?.addEventListener('click', () => setTheme(document.body.classList.contains('light-theme') ? 'dark' : 'light'));
  document.querySelector('#studio-panic')?.addEventListener('click', async () => {
    if (!window.confirm('Stop all mapped output now?')) return;
    if (announcement) announcement.textContent = 'Panic requested; waiting for daemon confirmation…';
    try {
      const response = await fetch('/api/v1/operations', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ request_id: `studio-panic-${Date.now()}`, operation: 'panic', confirm: true, payload: {} }) });
      if (!response.ok) throw new Error(`panic ${response.status}`);
      if (announcement) announcement.textContent = 'Panic confirmed. All mapped output was stopped.';
    } catch (_) {
      if (announcement) announcement.textContent = 'Panic could not be confirmed; review system status before continuing.';
    }
  });
  document.querySelectorAll('.layer-switcher button').forEach(button => button.addEventListener('click', () => {
    const label = button.textContent.trim();
    const turningOff = button.classList.contains('layer-active') && label !== 'Base';
    document.querySelectorAll('.layer-switcher button').forEach(item => { item.classList.remove('layer-active'); item.setAttribute('aria-pressed', 'false'); });
    if (turningOff) document.querySelector('.layer-switcher button:first-of-type')?.classList.add('layer-active');
    else button.classList.add('layer-active');
    document.querySelector('.layer-switcher button.layer-active')?.setAttribute('aria-pressed', 'true');
    announcement.textContent = turningOff ? `${label} layer turned off; Base is active.` : `${label} layer selected. Choose a control to inspect its assignments.`;
    state.publish({ layer: turningOff ? null : label });
    window.MackesStudioRefresh?.();
  }));
  document.querySelectorAll('.destination-preview button').forEach(button => button.addEventListener('click', () => {
    announcement.textContent = `${button.querySelector('strong')?.textContent || 'Device'} destination browser will open after a control is selected.`;
  }));
  document.querySelector('.add-assignment')?.addEventListener('click', () => {
    if (!state.read().selectedControl) return;
    window.MackesStudioCatalog?.open('pipedal');
    announcement.textContent = 'Choose another destination for this control.';
  });
  document.querySelector('#studio-starter-review')?.addEventListener('click', async () => {
    const status = document.querySelector('#studio-starter-status');
    const apply = document.querySelector('#studio-starter-apply');
    if (apply) { apply.hidden = true; apply.disabled = true; apply.dataset.generation = ''; }
    if (status) status.textContent = 'Checking connected devices and compatible functions…';
    try {
      const response = await fetch('/api/v1/studio/capabilities', { signal: AbortSignal.timeout(10000) });
      if (!response.ok) throw new Error(`capabilities ${response.status}`);
      const body = await response.json();
      const ready = Array.isArray(body.devices) ? body.devices.filter(device => device.lifecycle === 'ready') : [];
      const mappingResponse = await fetch('/api/v1/mappings', { signal: AbortSignal.timeout(10000) });
      const mappingBody = mappingResponse.ok ? await mappingResponse.json() : {};
      const occupied = new Set((Array.isArray(mappingBody.active) ? mappingBody.active : [])
        .filter(mapping => mapping.enabled !== false)
        .map(mapping => mapping.physical_control_id || mapping.physical_control));
      const controls = ['knob-r1-c1', 'knob-r1-c2', 'button-r1-c1', 'fader-1'];
      const availableControls = controls.filter(control => !occupied.has(control));
      const candidates = ready
        .filter(device => !/novation\.launch-control-xl/i.test(String(device.renderer || '')))
        .flatMap(device => (Array.isArray(device.features) ? device.features : [])
          .filter(feature => feature.writable)
          .map(feature => ({ device, feature })));
      const proposalCount = Math.min(candidates.length, availableControls.length);
      const proposals = document.querySelector('#studio-starter-proposals');
      if (proposals) {
        proposals.replaceChildren();
        candidates.slice(0, availableControls.length).forEach(({ device, feature }, index) => {
          const control = availableControls[index];
          const row = document.createElement('div'); row.className = 'starter-proposal';
          row.dataset.control = control; row.dataset.profile = device.renderer; row.dataset.device = device.stable_id; row.dataset.effect = device.renderer === 'eventide.micropitch' ? 'micropitch' : 'reflex'; row.dataset.parameter = feature.key;
          const name = document.createElement('strong'); name.textContent = `${control} → ${feature.label}`;
          const note = document.createElement('span'); note.textContent = `${device.label} · Preview only · ${feature.led_feedback ? 'LED feedback available' : 'No LED claim'}`;
          row.append(name, note); proposals.append(row);
        });
        proposals.hidden = candidates.length === 0;
        if (apply && proposalCount > 0) { apply.hidden = false; apply.disabled = false; apply.dataset.generation = String(Number.isInteger(mappingBody.generation) ? mappingBody.generation : 0); }
      }
      if (status) status.textContent = ready.length ? `${ready.length} supported device${ready.length === 1 ? '' : 's'} ready. Review ${proposalCount} proposed assignment${proposalCount === 1 ? '' : 's'} before applying; existing assignments were preserved.` : 'No supported devices are ready yet. Connect a device and try again.';
    } catch (_) {
      if (status) status.textContent = 'Device capabilities are unavailable. Nothing was changed; try again when the connection is ready.';
    }
  });
  document.querySelector('#studio-starter-apply')?.addEventListener('click', async event => {
    const button = event.currentTarget;
    const rows = Array.from(document.querySelectorAll('#studio-starter-proposals .starter-proposal')).filter(row => row.dataset.control);
    const generation = Number(button.dataset.generation);
    if (!rows.length || !Number.isInteger(generation)) return;
    if (!window.confirm(`Apply ${rows.length} reviewed assignment${rows.length === 1 ? '' : 's'} as one setup?`)) return;
    const sourceNumbers = { 'knob-r1-c1': 21, 'knob-r1-c2': 22, 'button-r1-c1': 41, 'fader-1': 77 };
    const mappings = rows.map(row => ({
      id: `quick-start-${row.dataset.control}-${row.dataset.profile}-${row.dataset.parameter}`.replace(/[^a-zA-Z0-9._-]/g, '-').slice(0, 64),
      controller_profile: 'launch-control-xl-mk2', physical_control_id: row.dataset.control,
      source_endpoint: 'controller', source_kind: row.dataset.control.startsWith('button-') ? 'note' : 'cc', source_channel: 0,
      destination_channel: null, source_number: sourceNumbers[row.dataset.control] || 0, destination_endpoint: row.dataset.device,
      destination_profile: row.dataset.profile, destination_effect: row.dataset.effect, destination_parameter: row.dataset.parameter,
      behavior: { source_range: [0, 127], destination_range: [0, 127], invert: false, curve: 'linear' }, enabled: true, profile_version: 1
    }));
    button.disabled = true;
    const status = document.querySelector('#studio-starter-status'); if (status) status.textContent = 'Applying the reviewed setup as one atomic transaction…';
    try {
      const response = await fetch('/api/v1/mappings', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ operation: 'Activate', generation, payload: { kind: 'Batch', mappings } }) });
      const body = await response.json().catch(() => ({}));
      if (!response.ok || body.outcome !== 'Applied') throw new Error(body.reason || `setup rejected (${response.status})`);
      if (status) status.textContent = `Setup applied atomically. ${mappings.length} assignment${mappings.length === 1 ? '' : 's'} saved.`;
      document.querySelector('#studio-starter-proposals')?.setAttribute('hidden', 'hidden');
      await window.MackesStudioRefresh?.();
    } catch (error) { if (status) status.textContent = `Setup was not changed: ${error.message || error}. Review the conflict and try again.`; button.disabled = false; }
  });
  const renderSelected = detail => {
    const title = document.querySelector('#assignment-title');
    const badge = document.querySelector('.sync-badge');
    const empty = document.querySelector('.panel-empty');
    const add = document.querySelector('.add-assignment');
    const mappings = state.read().assignments || [];
    const activeLayer = state.read().layer || 'Base';
    const entries = mappings.filter(item => {
      if ((item?.physical_control_id || item?.physical_control) !== detail.id) return false;
      const layer = item?.layer || item?.mapping_layer || item?.layer_id;
      return !layer || layer === activeLayer || (activeLayer === 'Base' && layer === 'base');
    });
    const active = entries.filter(item => item.enabled !== false);
    const first = active[0];
    const name = first ? friendly(first.destination_parameter || first.destination_effect || first.destination_profile) : 'Unassigned';
    if (title) title.textContent = detail.label.replace(/\b\w/g, character => character.toUpperCase());
    if (badge) badge.textContent = active.length ? 'Saved' : entries.length ? 'Disabled' : 'Unassigned';
    if (empty) empty.textContent = active.length ? `${name}${active.length > 1 ? ` and ${active.length - 1} more destination${active.length === 2 ? '' : 's'}` : ''} · current value follows the device.` : `${detail.label.replace(/\b\w/g, character => character.toUpperCase())} has no active destinations yet.`;
    if (add) add.disabled = false;
    const cards = document.querySelector('#studio-destination-cards');
    if (cards) {
      cards.replaceChildren();
      const observedCache = state.read().observed || {};
      active.forEach((mapping, index) => {
        const card = document.createElement('article'); card.className = 'destination-card';
        const heading = document.createElement('strong'); heading.textContent = friendly(mapping.destination_parameter || mapping.destination_effect || mapping.destination_profile || 'Destination');
        const detailText = document.createElement('span'); detailText.textContent = `${friendly(mapping.destination_profile || 'Device')}${mapping.behavior ? ` · ${mapping.behavior.curve || 'linear'} response` : ''}`;
        const observed = mapping.observed_value ?? observedCache[detail.id];
        const stateText = document.createElement('small'); stateText.textContent = observed !== undefined ? `Observed ${observed}` : 'Waiting for device value';
        card.append(heading, detailText, stateText); card.dataset.destinationIndex = String(index); cards.append(card);
      });
      cards.hidden = active.length === 0;
    }
    announcement.textContent = `${detail.label.replace(/\b\w/g, character => character.toUpperCase())} selected. Choose PiPedal, Eventide, or Lexicon to add a destination.`;
  };
  state.subscribe(next => {
    if (health && next.connection) health.textContent = next.connection === 'ready' ? 'Connected' : next.connection === 'offline' ? 'Daemon offline' : 'Connecting…';
    document.querySelectorAll('.physical-control').forEach(control => { const pending = next.pendingMutation?.controlId === control.dataset.controlId; const item = control.querySelector('.control-sync'); if (item) { item.textContent = pending ? 'Saving…' : next.connection === 'ready' ? 'Live sync' : next.connection === 'offline' ? 'Stale' : 'Sync pending'; item.classList.toggle('is-stale', next.connection === 'offline'); } });
    if (next.selectedControl) renderSelected(next.selectedControl);
  });
  document.querySelector('#studio-controller')?.addEventListener('studio-control-selected', event => renderSelected(event.detail));
  const applyMappings = body => {
    const mappings = Array.isArray(body?.mapping_registry) ? body.mapping_registry
      : Array.isArray(body?.active) ? body.active : Array.isArray(body?.control_mappings) ? body.control_mappings : [];
    const byControl = new Map();
    mappings.forEach(mapping => {
      const id = mapping?.physical_control_id || mapping?.physical_control;
      if (!id) return;
      const current = byControl.get(id) || [];
      current.push(mapping);
      byControl.set(id, current);
    });
    document.querySelectorAll('[data-control-id]').forEach(control => {
      const activeLayer = state.read().layer || 'Base';
      const entries = (byControl.get(control.dataset.controlId) || []).filter(item => {
        const layer = item?.layer || item?.mapping_layer || item?.layer_id;
        return !layer || layer === activeLayer || (activeLayer === 'Base' && layer === 'base');
      });
      const assignment = control.querySelector('.control-assignment');
      const value = control.querySelector('.control-value');
      const sync = control.querySelector('.control-sync');
      const active = entries.filter(item => item.enabled !== false);
      const first = active[0];
      const name = first ? friendly(first.destination_parameter || first.destination_effect || first.destination_profile) : 'Unassigned';
      const ledIntent = first?.led || first?.led_intent || 'unspecified';
      const observedCache = state.read().observed || {};
      const reported = first?.observed_value ?? first?.value ?? first?.last_sent_value ?? observedCache[control.dataset.controlId];
      if (assignment) assignment.textContent = `${active.length > 1 ? `${name} +${active.length - 1}` : name}${activeLayer !== 'Base' && active.length ? ` · ${activeLayer}` : ''}`;
      if (value && reported !== undefined && reported !== null) value.textContent = `${first?.observed_value !== undefined || observedCache[control.dataset.controlId] !== undefined ? 'Observed' : 'Last sent'} ${Number(reported).toFixed(2).replace(/\.00$/, '')}`;
      if (reported !== undefined && reported !== null) setVisualLevel(control, reported);
      control.classList.toggle('has-observed-value', observedCache[control.dataset.controlId] !== undefined || first?.observed_value !== undefined);
      if (sync) { sync.textContent = 'Live sync'; sync.classList.remove('is-stale'); }
      const pickup = Boolean(first?.pickup_required);
      if (sync && pickup) sync.textContent = 'Move to pickup';
      control.classList.toggle('needs-pickup', pickup);
      control.classList.toggle('is-assigned', active.length > 0);
      control.classList.toggle('is-disabled', entries.length > 0 && active.length === 0);
      control.dataset.ledIntent = ledIntent;
      control.classList.toggle('has-led-intent', ledIntent !== 'unspecified');
      control.setAttribute('aria-label', `${control.dataset.controlId}; ${control.classList.contains('physical-control-fader') ? 'no LED' : 'LED capable'}; ${active.length ? `${name}${active.length > 1 ? `; ${active.length} destinations` : ''}` : 'unassigned'}`);
    });
    state.publish({ generation: Number.isInteger(body?.generation) ? body.generation : state.read().generation, assignments: mappings, connection: 'ready', streamGap: false });
    document.querySelector('#studio-controller')?.classList.remove('is-stale');
    if (announcement) announcement.textContent = `${mappings.filter(item => item?.enabled !== false).length} saved assignment${mappings.filter(item => item?.enabled !== false).length === 1 ? '' : 's'} loaded.`;
  };
  const refresh = async () => {
    try {
      const response = await fetch('/api/v1/health', { signal: AbortSignal.timeout(2500) });
      if (!response.ok) throw new Error(`health ${response.status}`);
      state.publish({ connection: 'ready' });
      const mappings = await fetch('/api/v1/mappings', { signal: AbortSignal.timeout(5000) });
      if (!mappings.ok) throw new Error(`mappings ${mappings.status}`);
      applyMappings(await mappings.json());
      const monitor = await fetch('/api/v1/monitor', { signal: AbortSignal.timeout(2500) });
      if (monitor.ok) handleStudioEvent({ payload: await monitor.json() });
    } catch (_) {
      state.publish({ connection: 'offline' });
      document.querySelector('#studio-controller')?.classList.add('is-stale');
      if (announcement) announcement.textContent = 'Assignments could not be refreshed; known control labels remain visible.';
    }
  };
  window.MackesStudioRefresh = refresh;
  let gapRefreshPending = false;
  state.subscribe(next => {
    if (!next.streamGap || gapRefreshPending) return;
    gapRefreshPending = true;
    if (announcement) announcement.textContent = 'Live updates skipped a message; resynchronizing the current setup…';
    Promise.resolve(refresh()).finally(() => { gapRefreshPending = false; });
  });
  const handleStudioEvent = message => {
    const payload = message?.payload || message || {};
    const activity = payload?.last_activity || payload;
    const pipedalEvent = /pipedal/i.test(String(message?.kind || payload?.device || payload?.source || payload?.command || ''));
    if (pipedalEvent) {
      const feature = payload?.symbol || payload?.control || payload?.parameter;
      const value = payload?.observed_value ?? payload?.value;
      if (feature && value !== undefined && value !== null) {
        state.reconcile({ connection: 'ready', observations: { [`pipedal:${feature}`]: { value, source: 'external', freshness: 'observed' } } });
        announcement.textContent = `PiPedal reported an observed value for ${friendly(feature)}.`;
      }
    }
    const id = activity?.physical_control_id || activity?.physical_control;
    if (id && (payload?.action === 'ControlCaptured' || payload?.event === 'control_captured' || payload?.kind === 'ControlCaptured')) {
      const captured = Array.from(document.querySelectorAll('[data-control-id]')).find(item => item.dataset.controlId === id);
      if (captured) {
        captured.click();
        announcement.textContent = `${friendly(id)} captured from the Novation controller.`;
      }
    }
    const observed = activity?.observed_value ?? activity?.value;
    if (!id || observed === undefined || observed === null) return;
    const control = document.querySelector(`[data-control-id="${CSS.escape(id)}"]`);
    if (!control) return;
    const value = control.querySelector('.control-value');
    const stale = activity?.freshness === 'stale' || activity?.truth === 'stale';
    if (value) value.textContent = `${stale ? 'Stale' : 'Observed'} ${Number(observed).toFixed(2).replace(/\.00$/, '')}`;
    setVisualLevel(control, observed);
    control.classList.add('has-observed-value');
    document.querySelectorAll('.physical-control.is-live-active').forEach(item => item.classList.remove('is-live-active'));
    control.classList.remove('is-live-active');
    void control.offsetWidth;
    control.classList.add('is-live-active');
    const liveInput = document.querySelector('#studio-live-input');
    if (liveInput) {
      liveInput.textContent = `${friendly(id)} · ${Number(observed).toFixed(0)}`;
      liveInput.classList.add('is-active');
      window.setTimeout(() => liveInput.classList.remove('is-active'), 900);
    }
    const sync = control.querySelector('.control-sync');
    if (sync) { sync.textContent = stale ? 'Stale · refresh needed' : 'Observed'; sync.classList.toggle('is-stale', stale); }
    control.classList.remove('needs-pickup');
    if (announcement) announcement.textContent = `${friendly(id)} moved · value ${Number(observed).toFixed(0)}.`;
    state.publish({ observed: { ...(state.read().observed || {}), [id]: observed }, connection: 'ready' });
  };
  window.MackesStudioHandleEvent = handleStudioEvent;
  refresh();
  const browserSmoke = new URLSearchParams(window.location.search || window.location.hash.replace(/^#/, '?')).has('browser_smoke');
  const stream = typeof EventSource === 'function' && !browserSmoke ? new EventSource('/api/v1/events/stream?after_sequence=0') : null;
  window.MackesStudioStream = stream;
  stream?.addEventListener('open', () => {
    state.publish({ connection: 'ready' });
  });
  stream?.addEventListener('message', event => {
    try {
      const message = JSON.parse(event.data);
      handleStudioEvent(message);
    } catch (_) { /* Ignore unrelated bounded events. */ }
  });
  stream?.addEventListener('resnapshot', () => { refresh(); });
  stream?.addEventListener('error', () => {
    state.publish({ connection: 'offline' });
    if (announcement) announcement.textContent = 'Live device feedback paused; reconnecting without replaying writes…';
  });
  document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') refresh(); });
})();
