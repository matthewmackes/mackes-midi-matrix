(() => {
  const browser = document.querySelector('#studio-destination-browser');
  const list = document.querySelector('#destination-functions');
  const search = document.querySelector('#destination-search');
  const source = document.querySelector('#destination-source');
  const state = window.MackesStudioState;
  if (!browser || !list || !state) return;
  let profile = '';
  let entries = [];
  let selected = null;
  let opener = null;
  const friendly = value => String(value || '').replace(/[._:-]+/g, ' ').replace(/\b\w/g, character => character.toUpperCase());
  const render = () => {
    const query = search?.value.trim().toLowerCase() || '';
    list.replaceChildren();
    const controlType = state.read().selectedControl?.type;
    const visible = entries.filter(entry => (!entry.compatible || entry.compatible.includes(controlType)) && `${entry.label} ${entry.detail}`.toLowerCase().includes(query));
    if (!visible.length) {
      const empty = document.createElement('p'); empty.className = 'browser-empty'; empty.textContent = entries.length ? 'No functions match that search.' : 'This device catalog is unavailable.'; list.append(empty); return;
    }
    visible.forEach(entry => {
      const button = document.createElement('button');
      button.type = 'button'; button.className = 'function-choice'; button.setAttribute('role', 'option');
      const label = document.createElement('strong'); label.textContent = entry.label;
      const detail = document.createElement('span'); detail.textContent = `${entry.detail}${entry.meter ? ' · Read-only meter' : entry.readable === false ? ' · Sent — device does not confirm' : entry.writable === false ? ' · Read-only' : ' · Writable'}${entry.requiresConfirmation ? ' · Confirmation required' : ''}`;
      button.append(label, detail);
      button.addEventListener('click', () => {
        selected = entry;
        state.publish({ selectedDestination: { ...entry, profile } });
        document.dispatchEvent(new CustomEvent('studio-destination-picked', { detail: { ...entry, profile } }));
        document.querySelector('#studio-announcement').textContent = `${entry.label} selected from the ${friendly(profile)} catalog. Review the preview before saving.`;
        list.querySelectorAll('button').forEach(item => item.setAttribute('aria-selected', String(item === button)));
      });
      list.append(button);
    });
  };
  const entriesFromProfile = target => {
    const features = Array.isArray(target?.features) ? target.features : [];
    return features.map(feature => typeof feature === 'string'
      ? { label: feature, detail: 'Qualified capability', id: feature, compatible: /tap|active|bypass|reset|store|recall|trigger|patch|diagnostic/i.test(feature) ? ['channel-button', 'utility'] : ['knob', 'fader'] }
      : { label: feature.label || feature.control || 'Named function', detail: 'Qualified device capability', id: feature.control || feature.label, compatible: /tap|active|bypass|reset|store|recall|trigger|patch|diagnostic|algorithm selector/i.test(feature.label || feature.control || '') ? ['channel-button', 'utility'] : ['knob', 'fader'] });
  };
  const open = async requested => {
    if (!state.read().selectedControl) {
      document.querySelector('#studio-announcement').textContent = 'Select a Novation control before choosing a destination.';
      return;
    }
    profile = requested;
    selected = null;
    browser.hidden = false;
    if (source) source.textContent = 'Loading authoritative catalog…';
    entries = [];
    if (profile === 'pipedal') {
      try {
        const response = await fetch('/api/v1/pipedal', { signal: AbortSignal.timeout(5000) });
        const body = response.ok ? await response.json() : {};
        const catalog = body.catalog || body.pipedal_catalog;
      const controls = Array.isArray(catalog) ? catalog.flatMap(plugin => (plugin.controls || []).map(control => ({ ...control, plugin_name: plugin.name, plugin_uri: control.plugin_uri || plugin.uri }))) : (Array.isArray(catalog?.controls) ? catalog.controls : []);
        controls.forEach(control => entries.push({
          id: control.symbol || control.name || control.label,
          label: `${control.plugin_name || control.plugin || 'PiPedal'} · ${control.label || control.name || control.symbol || 'Control'}`,
          detail: control.units || (control.min_value !== undefined && control.max_value !== undefined ? `Range ${control.min_value}–${control.max_value}` : 'PiPedal control'),
          effect: control.plugin_uri || control.plugin || null,
          parameter: control.symbol || control.name || control.label,
          readable: true,
          writable: control.writable !== false,
          meter: control.writable === false,
          observedValue: control.value,
          compatible: /bypass|preset|snapshot|action|trigger|switch/i.test(control.symbol || control.name || control.label || '') ? ['channel-button', 'utility'] : ['knob', 'fader']
        }));
        state.publish({ pipedalCatalog: { controls: entries.length, writable: entries.filter(entry => entry.writable).length, meters: entries.filter(entry => entry.meter).length, connection: entries.length ? 'ready' : 'unavailable' } });
        if (source) source.textContent = entries.length ? 'PiPedal live catalog' : 'PiPedal catalog unavailable';
      } catch (_) { if (source) source.textContent = 'PiPedal catalog unavailable'; }
    } else {
      const target = window.MackesFeatureCatalog?.catalog?.find(item => item.profile === profile);
      entries = entriesFromProfile(target).map(entry => ({ ...entry, effect: profile === 'eventide.micropitch' ? 'micropitch' : 'reflex', parameter: entry.id, writable: true, readable: profile !== 'eventide.micropitch', requiresConfirmation: profile === 'lexicon.reflex' && /store|reset|diagnostic/i.test(entry.label || '') }));
      if (source) source.textContent = target?.source || 'Qualified profile catalog';
    }
    render();
  };
  document.querySelectorAll('.destination-preview button').forEach(button => button.addEventListener('click', () => {
    const name = button.querySelector('strong')?.textContent || '';
    const requested = /pipedal/i.test(name) ? 'pipedal' : /eventide/i.test(name) ? 'eventide.micropitch' : 'lexicon.reflex';
    opener = button;
    open(requested);
  }));
  search?.addEventListener('input', render);
  list.addEventListener('keydown', event => {
    const choices = Array.from(list.querySelectorAll('button.function-choice'));
    if (!choices.length || !['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const current = choices.indexOf(document.activeElement);
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? choices.length - 1 : (current + (event.key === 'ArrowDown' ? 1 : -1) + choices.length) % choices.length;
    choices[next].focus();
  });
  document.addEventListener('keydown', event => {
    if (event.key !== 'Escape' || browser.hidden) return;
    browser.hidden = true;
    document.querySelector('#studio-announcement').textContent = 'Destination browser closed. Your current assignment draft is unchanged.';
    opener?.focus();
  });
  window.MackesStudioCatalog = Object.freeze({ open, read: () => ({ profile, entries, selected }) });
})();
