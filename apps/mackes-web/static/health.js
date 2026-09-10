// Pure health presentation policy; transport and DOM ownership remain in app.js.
window.MackesHealth = Object.freeze({
  status({ failures, lastSuccessAt, now = Date.now() }) {
    const age = lastSuccessAt
      ? `${Math.max(0, Math.round((now - lastSuccessAt) / 1000))}s ago`
      : 'no successful check yet';
    if (failures >= 3) return { offline: true, text: `Backend offline · /api/v1/health · last success ${age} · retrying automatically` };
    return { offline: false, text: `Backend check delayed · /api/v1/health · last success ${age}` };
  }
});
