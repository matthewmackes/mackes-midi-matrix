(function () {
  function render(snapshot, parent) {
    const library = document.createElement('section'); library.className = 'pipedal-library-card'; library.setAttribute('aria-labelledby', 'pipedal-library-title');
    const heading = document.createElement('h2'); heading.id = 'pipedal-library-title'; heading.textContent = 'Pedalboards and snapshots'; library.append(heading);
    const note = document.createElement('p'); note.className = 'canvas-note'; note.textContent = 'Preview and recall actions appear only after their authoritative operation is qualified.'; library.append(note);
    const hierarchy = document.createElement('p'); hierarchy.className = 'library-hierarchy'; hierarchy.textContent = 'MACKES setlist → scene → mapping/layer · PiPedal pedalboard → snapshot → plugin control'; library.append(hierarchy);
    const search = document.createElement('input'); search.type = 'search'; search.className = 'library-search'; search.placeholder = 'Search pedalboards, snapshots, and controls'; search.maxLength = 96; search.setAttribute('aria-label', 'Search pedalboard library'); library.append(search);
    const catalog = snapshot?.catalog || snapshot?.pipedal_catalog || {};
    const groups = [
      ['Pedalboard targets', Array.isArray(catalog.targets) ? catalog.targets : [], 'target'],
      ['Named controls', Array.isArray(catalog.controls) ? catalog.controls : [], 'control'],
    ];
    const grid = document.createElement('div'); grid.className = 'pipedal-library-grid';
    const renderGroups = () => { grid.replaceChildren(); const query = search.value.trim().toLowerCase(); groups.forEach(([label, entries, kind]) => {
      const tile = document.createElement('article'); tile.className = 'pipedal-library-tile';
      const title = document.createElement('h3'); title.textContent = label; tile.append(title);
      const thumbnail = window.MackesModArt?.image?.(kind === 'target' ? 'rack' : 'library', { alt: `${label} artwork`, width: 120, height: 42 });
      if (thumbnail) { thumbnail.classList.add('library-thumbnail'); tile.append(thumbnail); }
      const filtered = entries.filter(entry => String(entry.name || entry.label || '').toLowerCase().includes(query));
      const body = document.createElement('p'); body.textContent = entries.length ? `${filtered.length} of ${entries.length} authoritative ${kind}${entries.length === 1 ? '' : 's'} available` : 'Unavailable in current snapshot'; tile.append(body);
      const badge = document.createElement('span'); badge.className = `library-state ${entries.length ? 'is-ready' : 'is-unavailable'}`; badge.textContent = entries.length ? 'Authoritative · current' : 'Unavailable · refresh'; tile.append(badge);
      if (filtered.length) { const list = document.createElement('ul'); filtered.slice(0, 8).forEach(entry => { const item = document.createElement('li'); item.textContent = String(entry.name || entry.label || 'Unnamed entry').slice(0, 96); list.append(item); }); tile.append(list); }
      const preview = document.createElement('button'); preview.type = 'button'; preview.className = 'quiet-button'; preview.textContent = 'Preview unavailable'; preview.disabled = true; preview.title = 'No qualified preview operation was reported'; tile.append(preview);
      grid.append(tile);
    }); };
    search.addEventListener('input', renderGroups); renderGroups();
    library.append(grid); parent.append(library); return library;
  }
  window.MackesPiPedalLibrary = Object.freeze({ render });
}());
