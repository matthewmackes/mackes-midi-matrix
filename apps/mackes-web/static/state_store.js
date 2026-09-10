window.MackesStateStore = (() => {
  let snapshot = Object.freeze({ view: 'state', generation: 0 });
  const listeners = new Set();
  return Object.freeze({
    read: () => snapshot,
    subscribe(listener) { listeners.add(listener); return () => listeners.delete(listener); },
    publish(next) {
      const generation = Number.isInteger(next?.generation) ? next.generation : snapshot.generation;
      if (generation < snapshot.generation) return false;
      snapshot = Object.freeze({ ...snapshot, ...next, generation });
      listeners.forEach(listener => listener(snapshot));
      return true;
    }
  });
})();
