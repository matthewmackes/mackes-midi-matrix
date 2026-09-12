(() => {
  const preview = document.querySelector('#studio-assignment-preview');
  const previewText = document.querySelector('#assignment-preview-text');
  const save = document.querySelector('#studio-assignment-save');
  const undo = document.querySelector('#studio-assignment-undo');
  const cancel = document.querySelector('#studio-assignment-cancel');
  const status = document.querySelector('#studio-assignment-status');
  const state = window.MackesStudioState;
  if (!preview || !previewText || !save || !state) return;
  let pending = null;
  const friendly = value => String(value || '').replace(/[._:-]+/g, ' ').replace(/\b\w/g, character => character.toUpperCase());
  let previewOpener = null;
  const close = () => { pending = null; state.publish({ draft: null }); preview.hidden = true; if (status) status.textContent = ''; previewOpener?.focus(); };
  document.addEventListener('studio-destination-picked', event => {
    const control = state.read().selectedControl;
    if (!control) return;
    pending = { control, destination: event.detail };
    previewOpener = document.querySelector('#destination-functions button[aria-selected="true"]');
    state.publish({ draft: pending });
    preview.hidden = false;
    previewText.textContent = `Moving ${control.label} will control ${event.detail.label} on ${friendly(event.detail.profile)}.`;
    if (status) status.textContent = 'Ready to save. The daemon will validate this assignment.';
    save.disabled = false;
    preview.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  });
  const restoredDraft = state.read().draft;
  if (restoredDraft?.control && restoredDraft?.destination) {
    pending = restoredDraft;
    preview.hidden = false;
    previewText.textContent = `Restored draft: moving ${restoredDraft.control.label} will control ${restoredDraft.destination.label} on ${friendly(restoredDraft.destination.profile)}.`;
    save.disabled = false;
  }
  cancel?.addEventListener('click', close);
  document.addEventListener('keydown', event => { if (event.key === 'Escape' && !preview.hidden) close(); });
  save.addEventListener('click', async () => {
    if (!pending || save.disabled) return;
    const { control, destination } = pending;
    save.disabled = true;
    state.publish({ pendingMutation: { kind: 'assignment', controlId: control.id } });
    if (status) status.textContent = 'Saving assignment…';
    try {
      const response = await fetch('/api/v1/assignment', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ generation: state.read().generation, action: 'Commit', physical_control_id: control.id, destination_profile: destination.profile, destination_effect: destination.effect || null, destination_parameter: destination.parameter || destination.id || null })
      });
      const body = await response.json().catch(() => ({}));
      if (!response.ok || body.applied === false) {
        if (response.status === 409) { status.textContent = 'Conflict detected. Refreshing authoritative state; your draft remains available.'; await window.MackesStudioRefresh?.(); throw new Error('assignment conflict; review the refreshed destination'); }
        throw new Error(body.reason || `assignment rejected (${response.status})`);
      }
      state.publish({ generation: Number.isInteger(body.generation) ? body.generation : state.read().generation, draft: null });
      if (undo) { undo.hidden = false; undo.disabled = false; }
      if (status) status.textContent = 'Saved. Refreshing the authoritative assignment state…';
      await window.MackesStudioRefresh?.();
      if (status) status.textContent = 'Saved and confirmed by the daemon.';
    } catch (error) {
      if (status) status.textContent = `Save needs attention: ${error.message || error}`;
      save.disabled = false;
    } finally {
      state.publish({ pendingMutation: null });
    }
  });
  undo?.addEventListener('click', async () => {
    if (undo.disabled || !window.confirm('Undo the last saved assignment?')) return;
    undo.disabled = true; if (status) status.textContent = 'Undoing the last assignment…';
    try {
      const response = await fetch('/api/v1/mappings', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ operation: 'Undo', generation: state.read().generation }) });
      const body = await response.json().catch(() => ({}));
      if (!response.ok || body.outcome !== 'Applied') throw new Error(body.reason || `undo rejected (${response.status})`);
      state.publish({ generation: Number.isInteger(body.generation) ? body.generation : state.read().generation });
      if (status) status.textContent = 'Last assignment undone and confirmed.';
      undo.hidden = true; await window.MackesStudioRefresh?.();
    } catch (error) { if (status) status.textContent = `Undo needs attention: ${error.message || error}`; undo.disabled = false; }
  });
  window.MackesStudioAssignment = Object.freeze({ close });
})();
