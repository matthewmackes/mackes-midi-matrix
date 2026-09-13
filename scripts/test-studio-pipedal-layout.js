#!/usr/bin/env node
const fs = require('fs');
const vm = require('vm');
const source = fs.readFileSync('apps/mackes-web/static/studio_pipedal_canvas.js', 'utf8');
const context = { window: {}, document: {} };
vm.runInNewContext(source, context);
const layout = context.window.MackesPiPedalCanvas.layoutTargets;
const targets = Array.from({ length: 80 }, (_, index) => ({ name: `Plugin ${index}`, instanceId: index }));
const first = layout(targets);
const second = layout(targets);
if (first.length !== 64 || JSON.stringify(first.map(item => [item.x, item.y])) !== JSON.stringify(second.map(item => [item.x, item.y]))) throw new Error('layout is not bounded and deterministic');
if (first.some(item => item.x < 20 || item.y < 20 || item.x > 545)) throw new Error('layout escaped graph bounds');
const reordered = layout([...targets].reverse());
if (reordered[0].target.instanceId !== 79 || reordered[0].x !== 20 || reordered[0].y !== 20) throw new Error('reordered graph did not receive deterministic positions');
console.log(`studio-pipedal-layout: PASS bounded=${first.length} stable=true reordered=true`);
