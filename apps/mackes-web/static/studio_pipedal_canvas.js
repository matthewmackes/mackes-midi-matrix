(function () {
  let selectedIdentity = null;
  const namespace = ['http', String.fromCharCode(58, 47, 47), 'www.w3.org/2000/svg'].join('');
  function text(parent, value, x, y, className) {
    const node = document.createElementNS(namespace, 'text');
    node.setAttribute('x', x); node.setAttribute('y', y); node.setAttribute('class', className); node.textContent = value; parent.append(node); return node;
  }
  function layoutTargets(targets, limit = 64) {
    return targets.slice(0, limit).map((target, index) => ({ target, x: 20 + (index % 4) * 175, y: 20 + Math.floor(index / 4) * 106 }));
  }
  function resolveConnections(targets, connections, limit = 128) {
    const identity = target => String(target.instance_id ?? target.instanceId ?? target.id ?? target.name);
    const known = new Set(targets.slice(0, 64).map(identity));
    return connections.slice(0, limit).filter(connection => {
      const from = String(connection.source_instance_id ?? connection.sourceInstanceId ?? connection.source_id ?? connection.source ?? '');
      const to = String(connection.destination_instance_id ?? connection.destinationInstanceId ?? connection.destination_id ?? connection.destination ?? '');
      return known.has(from) && known.has(to) && (connection.type || connection.kind);
    });
  }
  function reconcileSelection(selected, targets) {
    if (selected == null) return null;
    const identity = target => String(target.instance_id ?? target.instanceId ?? target.id ?? target.name);
    return targets.some(target => identity(target) === String(selected)) ? String(selected) : null;
  }
  function meterLabel(target) {
    const value = target?.meter_db ?? target?.meterDb ?? target?.level_db ?? target?.levelDb;
    return Number.isFinite(Number(value)) ? `Level ${Number(value).toFixed(1)} dB` : 'Level unavailable';
  }
  function render(snapshot, parent) {
    const catalog = snapshot?.catalog || snapshot?.pipedal_catalog || {};
    const targets = Array.isArray(catalog.targets) ? catalog.targets.filter(item => item && item.name) : [];
    const section = document.createElement('section'); section.className = 'pipedal-canvas-card'; section.setAttribute('aria-labelledby', 'pipedal-canvas-title');
    const heading = document.createElement('h2'); heading.id = 'pipedal-canvas-title'; heading.textContent = 'PiPedal signal flow'; section.append(heading);
    const note = document.createElement('p'); note.className = 'canvas-note'; note.textContent = targets.length ? `${targets.length} authoritative plugin target${targets.length === 1 ? '' : 's'} · connections reported only when available` : 'Pedalboard topology unavailable; no plugin targets were reported.'; section.append(note);
    if (!targets.length) { section.dataset.topology = 'unavailable'; parent.append(section); return section; }
    const svg = document.createElementNS(namespace, 'svg'); svg.classList.add('pipedal-canvas'); svg.setAttribute('viewBox', `0 0 720 ${Math.max(150, Math.ceil(targets.length / 4) * 106)}`); svg.setAttribute('role', 'img'); svg.setAttribute('aria-label', 'Authoritative PiPedal plugin target list');
    const viewport = document.createElement('div'); viewport.className = 'pipedal-viewport'; viewport.setAttribute('aria-label', 'PiPedal graph viewport');
    const controls = document.createElement('div'); controls.className = 'pipedal-viewport-controls'; controls.setAttribute('aria-label', 'Graph viewport controls');
    let scale = 1;
    const updateScale = () => { svg.style.transform = `scale(${scale})`; svg.style.transformOrigin = 'top left'; svg.style.marginBottom = `${(scale - 1) * 100}px`; status.textContent = `Graph zoom ${Math.round(scale * 100)} percent.`; };
    const status = document.createElement('span'); status.className = 'canvas-note'; status.setAttribute('role', 'status'); status.setAttribute('aria-live', 'polite'); status.textContent = 'Graph zoom 100 percent.';
    [['Zoom out', () => { scale = Math.max(.75, scale - .25); updateScale(); }], ['Zoom in', () => { scale = Math.min(2, scale + .25); updateScale(); }], ['Fit graph', () => { scale = 1; updateScale(); viewport.scrollTo({ left: 0, top: 0 }); }]].forEach(([label, action]) => { const button = document.createElement('button'); button.type = 'button'; button.className = 'quiet-button'; button.textContent = label; button.addEventListener('click', action); controls.append(button); });
    controls.append(status); section.append(controls);
    const legend = document.createElement('p'); legend.className = 'pipedal-cable-legend'; legend.textContent = 'Connection legend: Audio · solid green | MIDI · dashed gold | CV · dotted violet | MACKES control · dashed blue'; legend.setAttribute('aria-label', 'Connection type legend'); section.append(legend);
    const connections = Array.isArray(snapshot?.connections) ? snapshot.connections : Array.isArray(snapshot?.pipedal_connections) ? snapshot.pipedal_connections : [];
    const identity = target => String(target.instance_id ?? target.instanceId ?? target.id ?? target.name);
    const layout = layoutTargets(targets);
    const positions = new Map(layout.map(({ target, x, y }) => [identity(target), { x, y }]));
    resolveConnections(targets, connections).forEach(connection => {
      const from = positions.get(String(connection.source_instance_id ?? connection.sourceInstanceId ?? connection.source_id ?? connection.source));
      const to = positions.get(String(connection.destination_instance_id ?? connection.destinationInstanceId ?? connection.destination_id ?? connection.destination));
      if (!from || !to) return;
      const line = document.createElementNS(namespace, 'path'); line.setAttribute('d', `M ${from.x + 150} ${from.y + 36} C ${from.x + 165} ${from.y + 36}, ${to.x - 15} ${to.y + 36}, ${to.x} ${to.y + 36}`); line.setAttribute('class', `pipedal-cable is-${String(connection.type || connection.kind || 'unknown').toLowerCase()}`); line.setAttribute('aria-label', `${connection.type || connection.kind || 'Unknown'} connection`); svg.append(line);
    });
    targets.slice(0, 64).forEach((target, index) => {
      const x = 20 + (index % 4) * 175; const y = 20 + Math.floor(index / 4) * 106;
      const group = document.createElementNS(namespace, 'g'); group.setAttribute('tabindex', '0'); group.setAttribute('role', 'button'); group.setAttribute('aria-label', `PiPedal target ${target.name}, target topology ${connections.length ? 'reported' : 'connection unavailable'}`); group.dataset.instanceId = identity(target);
      if (selectedIdentity === identity(target)) group.setAttribute('aria-current', 'true');
      const box = document.createElementNS(namespace, 'rect'); box.setAttribute('x', x); box.setAttribute('y', y); box.setAttribute('width', 150); box.setAttribute('height', 72); box.setAttribute('rx', 10); box.setAttribute('class', 'pipedal-node'); group.append(box);
      text(group, String(target.name).slice(0, 24), x + 12, y + 27, 'pipedal-node-title');
      text(group, `Target · ${connections.length ? 'connections reported' : 'connection unavailable'} · ${meterLabel(target)}`, x + 12, y + 49, 'pipedal-node-state');
      const choose = () => {
        selectedIdentity = identity(target);
        group.setAttribute('aria-current', 'true');
        const destination = { label: String(target.name), profile: 'pipedal', effect: target.uri || target.name, parameter: target.symbol || target.parameter || target.name, writable: target.writable !== false };
        document.dispatchEvent(new CustomEvent('studio-destination-picked', { detail: { ...destination, sourceElement: group } }));
        const announcement = document.querySelector('#studio-announcement');
        if (announcement) announcement.textContent = 'PiPedal target selected. Choose a Novation control or review the assignment draft.';
      };
      group.addEventListener('click', choose); group.addEventListener('keydown', event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); choose(); } });
      svg.append(group);
    });
    if (!connections.length) { const unavailable = document.createElement('p'); unavailable.className = 'canvas-note'; unavailable.textContent = 'Connections unavailable in the authoritative snapshot; no cables are inferred.'; section.append(unavailable); }
    selectedIdentity = reconcileSelection(selectedIdentity, targets);
    viewport.append(svg); section.append(viewport);
    const list = document.createElement('ul'); list.className = 'pipedal-topology-list'; list.setAttribute('aria-label', 'PiPedal targets');
    targets.slice(0, 64).forEach(target => { const item = document.createElement('li'); item.textContent = `${target.name} · target present · ${connections.length ? 'connections reported' : 'connections unavailable'}`; list.append(item); });
    section.append(list); parent.append(section); return section;
  }
  window.MackesPiPedalCanvas = Object.freeze({ render, layoutTargets, resolveConnections, reconcileSelection, meterLabel });
}());
