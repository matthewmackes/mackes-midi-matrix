#!/usr/bin/env node
const fs = require('fs');
const vm = require('vm');
const context = { window: {}, document: {} };
vm.runInNewContext(fs.readFileSync('apps/mackes-web/static/mod_art.js', 'utf8'), context);
const key = context.window.MackesModArt.faceAssetKey;
if (key('pipedal') !== 'rack' || key('rack.generic') !== 'rack' || key('unknown.plugin') !== 'library') throw new Error('plugin-face fallback mapping is not deterministic');
console.log('mod-art-plugin-face: PASS native=rack generic=rack unknown=library');
