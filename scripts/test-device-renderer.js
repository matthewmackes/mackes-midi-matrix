#!/usr/bin/env node
const fs = require('fs');
const vm = require('vm');
const context = { window: {}, document: {}, Object, String, Math };
vm.createContext(context);
vm.runInContext(fs.readFileSync('apps/mackes-web/static/device_renderer.js', 'utf8'), context);
const renderer = context.window.MackesDeviceRenderer;
const cases = [
  ['Launch Control XL', 'novation.launch-control-xl'],
  ['Eventide MicroPitch', 'eventide.micropitch'],
  ['Lexicon Reflex', 'lexicon.reflex'],
  ['PiPedal', 'pipedal'],
  ['MIDISPORT 4x4', 'm-audio.midisport-4x4'],
  ['RTP-MIDI peer', 'rtp-midi'],
  ['MACKES virtual monitor', 'mackes.virtual-monitor'],
  ['mystery instrument', 'generic.endpoint'],
];
for (const [name, expected] of cases) {
  const actual = renderer.rendererFor({ name }).key;
  if (actual !== expected) throw new Error(`${name}: expected ${expected}, got ${actual}`);
}
if (renderer.keys.length !== 9 || new Set(renderer.keys).size !== renderer.keys.length) throw new Error('renderer registry is not unique and complete');
const stateCases = [
  [{ state: 'ready', input_count: 1, output_count: 1 }, 'Connected', 'MIDI in · MIDI out'],
  [{ state: 'degraded', input_count: 1 }, 'Needs attention', 'MIDI in'],
  [{ state: 'disconnected' }, 'Disconnected', 'MIDI direction unavailable'],
  [{ state: 'mystery' }, 'Unknown', 'MIDI direction unavailable'],
];
for (const [device, expectedLabel, expectedDirection] of stateCases) {
  const actual = renderer.stateModel(device);
  if (actual.label !== expectedLabel || actual.direction !== expectedDirection) throw new Error(`state matrix mismatch for ${device.state}`);
}
console.log(`device renderer checks passed (${renderer.keys.length} keys)`);
