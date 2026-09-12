window.MackesStudioState = (() => {
  let storedDraft = null;
  try {
    const parsed = JSON.parse(window.localStorage.getItem('mackes-studio-draft') || 'null');
    storedDraft = parsed?.control?.id && parsed?.destination?.profile ? parsed : null;
    if (!storedDraft && parsed) window.localStorage.removeItem('mackes-studio-draft');
  } catch (_) { storedDraft = null; }
  let snapshot = Object.freeze({
    view: 'controller', selectedControl: storedDraft?.control || null, generation: 0, sequence: 0,
    connection: 'connecting', authoritative: null, capabilities: null, observations: {}, streamGap: false,
    pendingWrites: {}, freshness: {}, focus: null, scroll: {}, draft: storedDraft
  });
  const listeners = new Set();
  const publish = next => {
    const generation = Number.isInteger(next?.generation) ? next.generation : snapshot.generation;
    if (generation < snapshot.generation) return false;
    if (Object.prototype.hasOwnProperty.call(next || {}, 'draft')) {
      try { if (next.draft) window.localStorage.setItem('mackes-studio-draft', JSON.stringify(next.draft)); else window.localStorage.removeItem('mackes-studio-draft'); } catch (_) { /* Storage may be unavailable; memory state remains valid. */ }
    }
    snapshot = Object.freeze({ ...snapshot, ...next, generation });
    listeners.forEach(listener => listener(snapshot));
    return true;
  };
  const reconcile = next => {
    const incomingGeneration = Number.isInteger(next?.generation) ? next.generation : snapshot.generation;
    const incomingSequence = Number.isInteger(next?.sequence) ? next.sequence : snapshot.sequence;
    if (incomingGeneration < snapshot.generation
      || (incomingGeneration === snapshot.generation && incomingSequence < snapshot.sequence)) return false;
    const patch = { ...next, generation: incomingGeneration, sequence: incomingSequence };
    if (next?.authoritative && typeof next.authoritative === 'object') {
      patch.authoritative = { ...(snapshot.authoritative || {}), ...next.authoritative };
    }
    if (next?.observations && typeof next.observations === 'object') {
      patch.observations = { ...snapshot.observations, ...next.observations };
    }
    if (next?.pendingWrites && typeof next.pendingWrites === 'object') {
      patch.pendingWrites = { ...snapshot.pendingWrites, ...next.pendingWrites };
    }
    return publish(patch);
  };
  const applyEvent = event => {
    if (!event || typeof event !== 'object') return false;
    const payload = event.payload && typeof event.payload === 'object' ? event.payload : {};
    const sequence = Number.isInteger(event.sequence) ? event.sequence : snapshot.sequence;
    const patch = { sequence, streamGap: sequence > snapshot.sequence + 1 };
    if (Number.isInteger(event.generation)) patch.generation = event.generation;
    if (event.device_id || event.feature_key) {
      const key = event.feature_key || event.device_id;
      patch.observations = { [key]: { ...payload, kind: event.kind || 'unknown', sequence: patch.sequence } };
    }
    return reconcile(patch);
  };
  return Object.freeze({ read: () => snapshot, publish, reconcile, applyEvent, subscribe(listener) { listeners.add(listener); return () => listeners.delete(listener); } });
})();
