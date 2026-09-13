#!/usr/bin/env node
const fs = require('fs');
const vm = require('vm');
const context = { window: {}, document: {} };
vm.runInNewContext(fs.readFileSync('apps/mackes-web/static/studio_pipedal_canvas.js', 'utf8'), context);
const meter = context.window.MackesPiPedalCanvas.meterLabel;
if (meter({ meter_db: -3.25 }) !== 'Level -3.3 dB') throw new Error('qualified meter was not formatted');
if (meter({ name: 'No telemetry' }) !== 'Level unavailable') throw new Error('missing meter was overstated');
console.log('studio-pipedal-meter: PASS qualified=true unavailable_truthful=true');
