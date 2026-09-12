(() => {
  const editor = document.querySelector('#studio-behavior-editor');
  const state = window.MackesStudioState;
  const fields = {
    sourceMin: document.querySelector('#studio-source-min'), sourceMax: document.querySelector('#studio-source-max'),
    destinationMin: document.querySelector('#studio-destination-min'), destinationMax: document.querySelector('#studio-destination-max'),
    curve: document.querySelector('#studio-curve'), invert: document.querySelector('#studio-invert'), led: document.querySelector('#studio-led-intent')
  };
  const save = document.querySelector('#studio-behavior-save');
  const reset = document.querySelector('#studio-behavior-reset');
  const status = document.querySelector('#studio-behavior-status');
  if (!editor || !state || !save) return;
  let selected = null;
  const currentMapping = () => (state.read().assignments || []).find(item => item?.id === selected?.mappingId);
  const setValues = behavior => {
    const value = behavior || {};
    fields.sourceMin.value = value.source_range?.[0] ?? 0;
    fields.sourceMax.value = value.source_range?.[1] ?? 127;
    fields.destinationMin.value = value.destination_range?.[0] ?? 0;
    fields.destinationMax.value = value.destination_range?.[1] ?? 127;
    fields.curve.value = value.curve || 'linear';
    fields.invert.checked = Boolean(value.invert);
  };
  const readBehavior = () => {
    const values = [fields.sourceMin, fields.sourceMax, fields.destinationMin, fields.destinationMax].map(input => Number(input.value));
    if (values.some(value => !Number.isInteger(value) || value < 0 || value > 16383) || values[0] > values[1] || values[2] > values[3]) throw new Error('Ranges must be whole numbers from 0 to 16383, with minimum before maximum.');
    return { source_range: [values[0], values[1]], destination_range: [values[2], values[3]], invert: fields.invert.checked, curve: fields.curve.value };
  };
  const announce = message => { if (status) status.textContent = message; };
  document.querySelector('#studio-controller')?.addEventListener('studio-control-selected', event => {
    const mappings = state.read().assignments || [];
    const matches = mappings.filter(item => (item?.physical_control_id || item?.physical_control) === event.detail.id && item.enabled !== false);
    const mapping = matches[0];
    selected = mapping?.id ? { mappingId: mapping.id, control: event.detail } : null;
    editor.hidden = !selected;
    save.disabled = !selected;
    if (selected) { setValues(mapping.behavior); announce('Adjust the response, then save explicitly.'); }
    else announce('Assign a destination first to edit behavior.');
  });
  reset?.addEventListener('click', () => { const mapping = currentMapping(); if (mapping) setValues(mapping.behavior); announce('Behavior reset to the last saved values.'); });
  save.addEventListener('click', async () => {
    if (!selected || save.disabled) return;
    let behavior;
    try { behavior = readBehavior(); } catch (error) { announce(error.message); return; }
    save.disabled = true; state.publish({ pendingMutation: { kind: 'behavior', controlId: selected.control?.id, mappingId: selected.mappingId } }); announce('Saving behavior…');
    try {
      const response = await fetch('/api/v1/mappings', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ operation: 'Behavior', generation: state.read().generation, payload: { kind: 'Behavior', mapping_id: selected.mappingId, behavior } }) });
      const body = await response.json().catch(() => ({}));
      if (!response.ok) {
        if (response.status === 409) { announce('Conflict detected. Refreshing authoritative behavior; your draft remains available.'); await window.MackesStudioRefresh?.(); throw new Error('behavior conflict; review the refreshed values'); }
        throw new Error(body.reason || `behavior rejected (${response.status})`);
      }
      state.publish({ generation: Number.isInteger(body.generation) ? body.generation : state.read().generation });
      announce('Saved. Refreshing the authoritative mapping…');
      await window.MackesStudioRefresh?.();
      announce('Behavior saved and confirmed by the daemon. LED intent remains a visual preview until LED projection is available.');
    } catch (error) { announce(`Save needs attention: ${error.message || error}`); }
    state.publish({ pendingMutation: null }); save.disabled = false;
  });
})();
