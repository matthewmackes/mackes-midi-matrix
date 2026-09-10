#!/usr/bin/env node
const fs = require('fs');
const vm = require('vm');

const source = fs.readFileSync('apps/mackes-web/static/state_store.js', 'utf8');
const context = { window: {} };
vm.runInNewContext(source, context, { filename: 'state_store.js' });
const store = context.window.MackesStateStore;
if (!store) throw new Error('state store was not initialized');
let calls = 0;
const unsubscribe = store.subscribe(snapshot => {
  calls += 1;
  if (snapshot.view !== 'devices') throw new Error('unexpected snapshot');
});
if (!store.publish({ view: 'devices', generation: 2, monitorPaused: true, routeDraftDirty: true })) throw new Error('publish failed');
if (store.publish({ view: 'state', generation: 1 })) throw new Error('stale generation accepted');
const snapshot = store.read();
if (snapshot.view !== 'devices' || !snapshot.monitorPaused || !snapshot.routeDraftDirty || calls !== 1) throw new Error('ordering or UI-state preservation failed');
unsubscribe();
store.publish({ view: 'routes', generation: 3 });
if (calls !== 1) throw new Error('unsubscribe failed');
console.log('state-store-smoke: PASS');
