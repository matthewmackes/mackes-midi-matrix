// Dependency-free navigation state boundary. Loaded before app.js so route parsing
// remains independently testable without introducing a build or CDN dependency.
window.MackesNavigation = Object.freeze({
  labels: Object.freeze({
    state: 'Live', mappings: 'Map Controls', assignment: 'Map Controls', routes: 'Routing',
    scenes: 'Scenes & Setlists', devices: 'Devices', recovery: 'Recovery', system: 'System', monitor: 'Monitor'
  }),
  viewFromLocation() {
    const segments = window.location.pathname.split('/').filter(Boolean);
    if (segments[0] === 'devices' && segments[1] === 'novation') return 'devices';
    if (segments[0] === 'system' && ['configuration', 'configuration/raw', 'backups', 'diagnostics'].includes(segments.slice(1).join('/'))) return 'system';
    const view = segments[0] || 'state';
    return Object.prototype.hasOwnProperty.call(this.labels, view) ? view : 'state';
  }
});
