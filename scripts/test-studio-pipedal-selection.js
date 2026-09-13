#!/usr/bin/env node
const fs = require('fs');
const vm = require('vm');
const context = { window: {}, document: {} };
vm.runInNewContext(fs.readFileSync('apps/mackes-web/static/studio_pipedal_canvas.js', 'utf8'), context);
const reconcile = context.window.MackesPiPedalCanvas.reconcileSelection;
const targets = [{ instanceId: 4, name: 'A' }, { instanceId: 5, name: 'B' }];
if (reconcile('5', targets) !== '5') throw new Error('existing selection was not retained');
if (reconcile('9', targets) !== null) throw new Error('replaced selection was not cleared');
console.log('studio-pipedal-selection: PASS retained=true replacement_cleared=true');
