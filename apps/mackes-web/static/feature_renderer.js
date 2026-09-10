window.MackesFeatureRenderer = Object.freeze({
  searchableText(entry) {
    const features = Array.isArray(entry?.features) ? entry.features : [];
    return `${entry?.name || ''} ${entry?.source || ''} ${features.map(feature =>
      typeof feature === 'string' ? feature : `${feature?.label || ''} CC ${feature?.cc ?? ''}`
    ).join(' ')}`.toLowerCase();
  },
  filter(entries, query) {
    const normalized = String(query || '').trim().toLowerCase();
    return (Array.isArray(entries) ? entries : []).filter(entry =>
      !normalized || this.searchableText(entry).includes(normalized)
    );
  }
});
