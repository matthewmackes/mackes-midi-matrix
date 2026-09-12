(() => {
  const view = window.MackesStudioState?.read().view;
  const panel = document.querySelector('#studio-supporting-view');
  if (!panel || !view || view === 'controller') return;
  const friendly = value => String(value || '').replace(/[._:-]+/g, ' ').replace(/\b\w/g, character => character.toUpperCase());
  const card = (title, value, detail) => { const article = document.createElement('article'); article.className = 'support-card'; const heading = document.createElement('h2'); heading.textContent = title; const main = document.createElement('strong'); main.textContent = value; const note = document.createElement('p'); note.textContent = detail; article.append(heading, main, note); return article; };
  const safeFetch = (url, timeout) => fetch(url, { signal: AbortSignal.timeout(timeout) }).catch(() => null);
  const render = async () => {
    panel.hidden = false; panel.replaceChildren(card('Loading', 'Checking studio state…', 'Authoritative status will appear here.'));
    try {
      const [healthResponse, mappingsResponse, pipedalResponse, scenesResponse, capabilitiesResponse] = await Promise.all([safeFetch('/api/v1/health', 5000), safeFetch('/api/v1/mappings', 10000), view === 'devices' ? safeFetch('/api/v1/pipedal', 10000) : Promise.resolve(null), view === 'scenes' ? safeFetch('/api/v1/scenes', 15000) : Promise.resolve(null), view === 'devices' ? safeFetch('/api/v1/studio/capabilities', 10000) : Promise.resolve(null)]);
      const health = healthResponse?.ok ? await healthResponse.json().catch(() => ({})) : {};
      const mappingBody = mappingsResponse?.ok ? await mappingsResponse.json().catch(() => ({})) : {};
      const pipedal = pipedalResponse?.ok ? await pipedalResponse.json().catch(() => ({})) : {};
      const scenesBody = scenesResponse?.ok ? await scenesResponse.json().catch(() => ({})) : {};
      const capabilitiesBody = capabilitiesResponse?.ok ? await capabilitiesResponse.json().catch(() => ({})) : {};
      const pipedalCatalog = pipedal.catalog || pipedal.pipedal_catalog || {};
      const pipedalCount = Array.isArray(pipedalCatalog.controls) ? pipedalCatalog.controls.length : 0;
      const mappings = Array.isArray(mappingBody.mapping_registry) ? mappingBody.mapping_registry : Array.isArray(mappingBody.active) ? mappingBody.active : [];
      const active = mappings.filter(item => item?.enabled !== false).length;
      panel.replaceChildren();
      if (view === 'devices') {
        panel.append(card('Novation', 'Launch Control XL', `${active} active assignment${active === 1 ? '' : 's'} · ${healthResponse?.ok ? 'connected' : 'connection unavailable'}`), card('PiPedal', pipedalCount ? `${pipedalCount} named controls` : 'Catalog unavailable', 'Browse current plugins and named controls from the Assignment panel.'), card('Eventide + Lexicon', 'Qualified profiles', 'MicroPitch and Reflex functions remain available through the same assignment flow.'));
        const devices = Array.isArray(capabilitiesBody.devices) ? capabilitiesBody.devices : [];
        const capabilityList = document.createElement('div'); capabilityList.className = 'device-capability-list'; capabilityList.setAttribute('aria-label', 'Connected device capabilities');
        devices.filter(device => !/novation/i.test(String(device.renderer || ''))).forEach(device => {
          const features = Array.isArray(device.features) ? device.features : [];
          const article = document.createElement('article'); article.className = 'support-card';
          const heading = document.createElement('h2'); heading.textContent = device.label || friendly(device.renderer); article.append(heading);
          if (window.MackesDeviceRenderer?.appendGraphic) window.MackesDeviceRenderer.appendGraphic({ name: device.label, kind: device.renderer, state: device.lifecycle }, article);
          const state = document.createElement('strong'); state.textContent = device.lifecycle === 'ready' ? 'Ready' : friendly(device.lifecycle || 'Unknown'); article.append(state);
          const summary = document.createElement('p'); const writable = features.filter(feature => feature.writable).length; const readable = features.filter(feature => feature.readable).length; summary.textContent = features.length ? `${features.length} named functions · ${writable} adjustable · ${readable} with live readback.` : 'No qualified functions are available yet.'; article.append(summary);
          if (features.length) { const list = document.createElement('ul'); features.slice(0, 8).forEach(feature => { const item = document.createElement('li'); item.textContent = `${feature.label}: ${feature.readable ? 'live feedback' : 'send only'}${feature.writable ? '' : ' · read only'}`; list.append(item); }); article.append(list); }
          capabilityList.append(article);
        });
        if (capabilityList.children.length) panel.append(capabilityList);
      } else if (view === 'routing') {
        panel.append(card('Control paths', `${active} active`, 'Each path begins at a named Novation control and ends at a selected device function.'), card('Sync', Number.isInteger(mappingBody.generation) ? `Generation ${mappingBody.generation}` : 'Awaiting generation', 'The daemon remains authoritative for ordering and conflict handling.'));
      } else if (view === 'scenes') {
        const scenes = Array.isArray(scenesBody.scenes) ? scenesBody.scenes : [];
        panel.append(card('Base scene', scenesBody.active_scene || 'Ready', scenesBody.active_scene ? 'The daemon reports the currently selected performance scene.' : 'Layer selection stays available from the Controller workspace.'));
        panel.append(card('Saved scenes', scenes.length ? `${scenes.length} available` : 'No saved scenes yet', scenes.length ? 'Choose a scene to review its destinations before recall.' : 'Your current assignments remain available; saving a scene will appear here.'));
        panel.append(card('Assignments', `${active} active`, 'Scene changes preserve destination details and clearly identify unconfirmed device portions.'));
        const actions = document.createElement('div'); actions.className = 'scene-actions'; actions.setAttribute('aria-label', 'Scene actions');
        const name = document.createElement('input'); name.type = 'text'; name.placeholder = 'Name this scene'; name.maxLength = 64; name.setAttribute('aria-label', 'New scene name');
        const save = document.createElement('button'); save.type = 'button'; save.className = 'add-assignment'; save.textContent = 'Save current setup';
        const status = document.createElement('p'); status.className = 'assignment-status'; status.setAttribute('role', 'status'); status.setAttribute('aria-live', 'polite');
        save.addEventListener('click', async () => {
          const scene = name.value.trim(); if (!scene) { status.textContent = 'Enter a scene name before saving.'; name.focus(); return; }
          save.disabled = true; status.textContent = 'Saving this setup…';
          try { const response = await fetch('/api/v1/scenes', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ scene, actions: [] }) }); if (!response.ok) throw new Error(`save ${response.status}`); status.textContent = `Saved “${scene}”.`; name.value = ''; await render(); }
          catch (_) { status.textContent = 'Scene could not be saved; your current setup is unchanged.'; save.disabled = false; }
        });
        actions.append(name, save);
        if (scenes.length) {
          const navigate = async (direction) => {
            status.textContent = `${direction === 'next' ? 'Moving to next' : 'Returning to previous'} scene…`;
            try {
              const response = await fetch('/api/v1/scenes', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ direction }) });
              if (!response.ok) throw new Error(`navigate ${response.status}`);
              status.textContent = 'Scene changed.';
              await render();
            } catch (_) { status.textContent = 'Scene change was not confirmed; review the current setup.'; }
          };
          const previous = document.createElement('button'); previous.type = 'button'; previous.className = 'quiet-button'; previous.textContent = 'Previous scene'; previous.addEventListener('click', () => navigate('previous'));
          const next = document.createElement('button'); next.type = 'button'; next.className = 'quiet-button'; next.textContent = 'Next scene'; next.addEventListener('click', () => navigate('next'));
          const recall = document.createElement('button'); recall.type = 'button'; recall.className = 'quiet-button'; recall.textContent = 'Recall selected scene'; recall.disabled = true;
          const select = document.createElement('select'); select.setAttribute('aria-label', 'Scene to recall');
          scenes.forEach(item => { const id = typeof item === 'string' ? item : item.id || item.name; if (!id) return; const option = document.createElement('option'); option.value = id; option.textContent = typeof item === 'string' ? item : item.name || id; select.append(option); });
          const rememberedScene = window.localStorage?.getItem('mackes-studio-selected-scene');
          if (rememberedScene && Array.from(select.options).some(option => option.value === rememberedScene)) select.value = rememberedScene;
          recall.disabled = !select.options.length;
          select.addEventListener('change', () => { recall.disabled = !select.value; if (select.value) window.localStorage?.setItem('mackes-studio-selected-scene', select.value); });
          if (select.value) window.localStorage?.setItem('mackes-studio-selected-scene', select.value);
          recall.addEventListener('click', async () => { if (!select.value || !window.confirm(`Recall “${select.value}”?`)) return; recall.disabled = true; status.textContent = 'Recalling scene…'; try { const response = await fetch('/api/v1/scenes', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ execute_scene: select.value }) }); if (!response.ok) throw new Error(`recall ${response.status}`); status.textContent = `“${select.value}” recalled.`; } catch (_) { status.textContent = 'Scene recall was not confirmed; review the current setup.'; recall.disabled = false; } });
          actions.append(previous, next, select, recall);
        }
        actions.append(status); panel.append(actions);
      } else {
        panel.append(card('Daemon', healthResponse?.ok ? 'Connected' : 'Unavailable', 'Connection state is reported from the health boundary.'), card('Recovery', 'Ready for reconnect', 'The clean-sheet UI preserves known labels and last observed values while offline.'), card('Advanced controls', 'Hidden by default', 'Unsupported protocol details remain outside the normal musician workflow.'));
      }
      const back = document.createElement('a'); back.className = 'back-link'; back.href = '/studio'; back.textContent = 'Open controller assignments →'; panel.append(back);
    } catch (_) { panel.replaceChildren(card('Studio state', 'Unavailable', 'Retry after the daemon connection returns; no local state was discarded.')); const retry = document.createElement('button'); retry.className = 'retry-link'; retry.type = 'button'; retry.textContent = 'Retry status refresh'; retry.addEventListener('click', render); const back = document.createElement('a'); back.className = 'back-link'; back.href = '/studio'; back.textContent = 'Open controller assignments →'; panel.append(retry, back); }
  };
  window.MackesStudioViewsRefresh = render;
  render();
})();
