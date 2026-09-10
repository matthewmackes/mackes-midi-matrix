#!/usr/bin/env node
const fs = require('fs');
const vm = require('vm');

const source = fs.readFileSync('apps/mackes-web/static/feature_renderer.js', 'utf8');
const context = { window: {} };
vm.runInNewContext(source, context, { filename: 'feature_renderer.js' });
const renderer = context.window.MackesFeatureRenderer;
if (!renderer) throw new Error('renderer was not initialized');
const entries = [
  { name: 'Eventide MicroPitch', source: 'manual', features: [{ label: 'Mix', cc: 20 }] },
  { name: 'Novation Launch Control', source: 'profile', features: ['24 knobs'] },
];
if (renderer.filter(entries, 'cc 20').length !== 1) throw new Error('CC search failed');
if (renderer.filter(entries, 'novation').length !== 1) throw new Error('name search failed');
if (renderer.filter(entries, 'missing').length !== 0) throw new Error('empty search failed');
console.log('feature-renderer-smoke: PASS');
