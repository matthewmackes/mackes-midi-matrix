#!/usr/bin/env node
const fs = require('fs');
const vm = require('vm');
const values = new Map();
const context = {
  window: { localStorage: { getItem: key => values.get(key) || null, setItem: (key, value) => values.set(key, value), removeItem: key => values.delete(key) } },
  JSON, Object, Number, Set, Map, console
};
vm.createContext(context);
vm.runInContext(fs.readFileSync('apps/mackes-web/static/studio_state.js', 'utf8'), context);
const store = context.window.MackesStudioState;
if (!store.reconcile({ generation: 2, sequence: 4, authoritative: { a: 1 }, draft: { control: { id: 'knob-1' }, destination: { profile: 'eventide.micropitch' } } })) throw new Error('initial reconcile rejected');
if (store.reconcile({ generation: 1, sequence: 5, authoritative: { stale: true } })) throw new Error('stale generation accepted');
if (!store.applyEvent({ generation: 2, sequence: 5, kind: 'control_observation', feature_key: 'knob-1', payload: { value: 91 } })) throw new Error('event rejected');
if (!store.applyEvent({ generation: 2, sequence: 6, kind: 'control_observation', feature_key: 'pipedal:gain', payload: { value: 0.75, source: 'external' } })) throw new Error('external event rejected');
if (!store.applyEvent({ generation: 2, sequence: 8, kind: 'control_observation', feature_key: 'knob-1', payload: { value: 92 } }) || !store.read().streamGap) throw new Error('event gap was not marked for resync');
if (!store.reconcile({ generation: 3, sequence: 1, connection: 'reconnected', pendingWrites: { 'knob-1': 'pending' }, focus: 'knob-1', scroll: { controller: 240 } })) throw new Error('reconnect snapshot rejected');
if (store.reconcile({ generation: 3, sequence: 0, authoritative: { stale: true } })) throw new Error('stale reconnect sequence accepted');
const state = store.read();
if (state.authoritative.a !== 1 || state.observations['knob-1'].value !== 92 || state.observations['pipedal:gain'].value !== 0.75 || state.draft.control.id !== 'knob-1' || state.pendingWrites['knob-1'] !== 'pending' || state.scroll.controller !== 240) throw new Error('reconciliation lost state');
console.log('studio state store checks passed');
