#!/usr/bin/env node
const fs = require('fs');
const vm = require('vm');
const context = { window: {}, document: {} };
vm.runInNewContext(fs.readFileSync('apps/mackes-web/static/studio_pipedal_canvas.js', 'utf8'), context);
const resolve = context.window.MackesPiPedalCanvas.resolveConnections;
const targets = [{ instanceId: 1, name: 'Input' }, { instanceId: 2, name: 'Effect' }, { instanceId: 3, name: 'Output' }];
const edges = resolve(targets, [
  { sourceInstanceId: 1, destinationInstanceId: 2, type: 'audio' },
  { source_instance_id: 2, destination_instance_id: 3, kind: 'midi' },
  { sourceInstanceId: 1, destinationInstanceId: 99, type: 'cv' },
  { sourceInstanceId: 1, destinationInstanceId: 2 },
]);
if (edges.length !== 2 || edges[0].type !== 'audio' || edges[1].kind !== 'midi') throw new Error('connection resolver fabricated or dropped a valid typed edge');
console.log(`studio-pipedal-connections: PASS valid=${edges.length} malformed_dropped=true typed=true`);
