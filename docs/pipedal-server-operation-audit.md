# PiPedal server operation audit

Research checkpoint for W145/W150, 2026-09-07. Enumerated all 104 message
registrations in local src/PiPedalSocket.cpp, compared with 42 connector Operation variants.
This file inventories exposed handlers, not qualified end-to-end functionality.
Plugin-specific metadata still needs separate review. The HTTP route and outbound-notification
surfaces are inventoried below; their payload and browser qualification remain W150 work.

Source: sibling pipedal checkout commit 859183d0d9614372318680433326e6c94c0251e5.
PiPedalSocket.cpp has no local changes and SHA-256
09691576451b502849db9028c4e22fbba2a9f9b2105b40f01ad049c8d7ccdc61.
Source-first developer evidence: `docs/Architecture.md` at the same checkout commit, SHA-256
`54e3c6f68b9c8d397d88d7dee6af978e927f86b9ce7bf006b6e0b70b8011278a`, documents the
asynchronous JSON-over-WebSocket request/event model and privileged service split. Matching
client sources `vite/src/pipedal/PiPedalModel.tsx` and `vite/src/pipedal/PiPedalSocket.tsx`
have SHA-256 `af961447f794aad616c61e75385b81e80027e4061c8eb221bf89ec638030a1a4` and
`29c93095aa8623b1650bcf745fe711cc8061c9c2427ea1542b4b06347b62286b`. These are
source-derived facts; they do not prove hardware/plugin readback or browser acceptance.
Connector source SHA-256:
097c255dca356a6a882f7507013c35e595694cb0f0dbe7fa73f6d9a2198fc2bf.
The locally installed `/usr/bin/pipedald` (the same inode as systemd's `/usr/sbin/pipedald`)
is not owned by an RPM. Its SHA-256 is
`08b54c4f5f4f87c9c1ac732e7b5db8f0367bee4a03d6f1f9227e7b58f5b758ba`, and its embedded
version strings are `PiPedal v2.0.110-Release` and `PiPedal v2.0.110`. This matches the pinned
checkout's CMake project/display version (`2.0.110` / `PiPedal v2.0.110-Release`). The checkout's
Release build artifact at `build/src/pipedald` is byte-for-byte identical to the installed
executable, including GNU build ID `357ed0938b81b0530351ccc38cfa66cd770ba2ab`; the tracked
checkout is clean, CMake records that checkout as `CMAKE_HOME_DIRECTORY`, and a dry-run build
reports `ninja: no work to do`. This establishes local build-tree-to-installed-binary provenance
at commit `859183d`. It is not a clean-room reproducible-build claim.

Evidence order for this audit is manufacturer/vendor documentation where available, then
developer documentation and matching source, then read-only fixtures. Binary inspection is used
only for version/build provenance; undocumented message semantics are not inferred from it.

Runtime boundary observed 2026-09-07: the pinned PiPedal 2.0.110 service auto-restarted after a
plain HTTP request to its WebSocket endpoint (`/pipedal`) caused
`std::filesystem::filesystem_error: cannot get file size: /etc/pipedal/react/pipedal` in
`WebServerImpl::on_http`. This is vendor/runtime evidence from `journalctl`, not a MACKES
protocol claim; normal MACKES health and bounded IPC behavior recovered after systemd restart.
Do not use a raw HTTP probe as a PiPedal WebSocket fixture.

## HTTP and server-event inventory

`src/WebServerConfig.cpp` SHA-256
`28e860b4d5e86604c7366872faf8e5bfe72abfaa6931c46191605ab1c3954917` contains 24 distinct
path segments handled outside the WebSocket request catalog:

```text
AudioMetadata, GetBank, GetPluginInfo, GetPluginPresets, GetPreset, NextAudioFile,
PluginBank, PluginBanks, PluginPreset, PluginPresets, PreviousAudioFile, Thumbnail,
displayMediaFile, downloadBank, downloadMediaFile, downloadPluginPresets, downloadPreset,
t3k_response.html, t3k_uploadAsset, tone3000_thumbnail, uploadBank, uploadPluginPresets,
uploadPreset, uploadUserFile
```

These routes cover media inspection/navigation, thumbnails, preset/plugin-preset/bank
import/export, user-file upload/download, and Tone3000 assets. Upload, import, and persistent
library actions require explicit confirmation, bounded payloads, path validation, and
non-production fixtures before W150 may expose them. Download/display and metadata operations
must retain the server's upload-directory and `..` rejection boundaries.

The pinned `PiPedalSocket.cpp` contains 37 distinct unsolicited `on*` event names:

```text
onAlsaSequencerConfigurationChanged, onBanksChanged, onChannelSelectionChanged,
onControlChanged, onErrorMessage, onFavoritesChanged, onGovernorSettingsChanged,
onHasWifiChanged, onInputVolumeChanged, onItemEnabledChanged, onJackConfigurationChanged,
onJackServerSettingsChanged, onLoadPluginPreset, onLv2PluginsChanging, onLv2StateChanged,
onNetworkChanging, onNotifyMidiListener, onNotifyPathPatchPropertyChanged,
onOutputVolumeChanged, onPatchPropertyChanged, onPedalboardChanged,
onPluginPresetsChanged, onPresetChanged, onPresetsChanged, onSelectedSnapshotChanged,
onShowStatusMonitorChanged, onSnapshotModified, onSystemMidiBindingsChanged,
onTone3000DownloadComplete, onTone3000DownloadError, onTone3000DownloadProgress,
onTone3000DownloadStarted, onUpdateStatusChanged, onUseItemModUiChanged,
onVst3ControlChanged, onWifiConfigSettingsChanged, onWifiDirectConfigSettingsChanged
```

This closes the inventory omission, not the behavioral qualification. W150 must correlate each
user-addressable request with its direct reply (if any), authoritative event(s), subscription
lifetime, reconnect behavior, and bounded queue policy. In particular, control and volume writes
may converge through events rather than direct replies; browser state must not fabricate an ACK.

Source-derived response census for the pinned `PiPedalSocket.cpp`: 65 of 104 handlers emit a
direct `Reply(replyTo, ...)`; 39 do not. Of those 39 send/event-oriented handlers, 37 invoke the
model and two are protocol helpers without a model call. The no-reply set is explicitly retained
in the per-operation inventory rather than being treated as a failed request: W150 must identify
the authoritative event or subscription completion for each, or classify it as an internal/helper
operation before exposing it in the browser.

## Per-operation inventory

Each stable feature ID is pipedal.<handler>. All rows use the source pin above.
“Catalogued” means enum membership only; payload, compatibility, permissions, persistence,
notifications and browser acceptance remain to be reconciled individually.
UI destination below is a proposed design home. Technical helper operations may support a
user-facing workflow without needing an independent button; require a documented disposition.

| Handler / feature suffix | Connector catalog | Proposed UI home | Qualification |
|---|---|---|---|
| setControl | catalogued | Pedalboard and parameter inspector | pending W150 |
| makeTone3000Pkce | missing | Device assets/library | pending W150 |
| writeTone3000Readme | missing | Device assets/library | pending W150 |
| sha256Base64url | missing | Device assets/library | pending W150 |
| previewControl | catalogued | Pedalboard and parameter inspector | pending W150 |
| setInputVolume | catalogued | Pedalboard and parameter inspector | pending W150 |
| setOutputVolume | catalogued | Pedalboard and parameter inspector | pending W150 |
| previewInputVolume | catalogued | Pedalboard and parameter inspector | adapter/browser preview path implemented; full installed qualification pending W150 |
| previewOutputVolume | catalogued | Pedalboard and parameter inspector | adapter/browser preview path implemented; full installed qualification pending W150 |
| listenForMidiEvent | catalogued | Monitoring and MIDI | bounded adapter/daemon queue path; event readback pending W150 |
| cancelListenForMidiEvent | catalogued | Monitoring and MIDI | bounded adapter/daemon queue path; event readback pending W150 |
| monitorPatchProperty | catalogued | Monitoring and MIDI | bounded adapter/daemon queue path; event readback pending W150 |
| cancelMonitorPatchProperty | catalogued | Monitoring and MIDI | bounded adapter/daemon queue path; event readback pending W150 |
| getUpdateStatus | catalogued | Device system settings | bounded nested readback and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| getHasWifi | catalogued | Device system settings | bounded startup readback and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| updateNow | catalogued | Device system settings | confirmed bounded adapter write implemented; daemon/browser/installed qualification pending W150 |
| getJackStatus | catalogued | Device system settings | bounded response decoder and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| getAlsaDevices | catalogued | Device system settings | bounded response decoder and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| getKnownWifiNetworks | catalogued | Device system settings | bounded startup readback and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| getWifiChannels | catalogued | Device system settings | bounded response decoder and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| getPluginPresets | catalogued | Presets and snapshots | bounded response decoder and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| loadPluginPreset | catalogued | Presets and snapshots | pending W150 |
| setJackServerSettings | catalogued | Device system settings | confirmed bounded adapter write implemented; daemon/browser/installed qualification pending W150 |
| setGovernorSettings | catalogued | Device system settings | pending W150 |
| setWifiConfigSettings | missing | Device system settings | pending W150 |
| getWifiConfigSettings | missing | Device system settings | pending W150 |
| setWifiDirectConfigSettings | missing | Device system settings | pending W150 |
| getWifiDirectConfigSettings | missing | Device system settings | pending W150 |
| getGovernorSettings | catalogued | Device system settings | bounded startup readback and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| getJackServerSettings | catalogued | Device system settings | bounded response decoder and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| getBankIndex | catalogued | Presets and snapshots | bounded readback implemented; full browser/installed qualification pending W150 |
| getJackConfiguration | catalogued | Device system settings | bounded response decoder and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| getJackSettings | catalogued | Device system settings | bounded response decoder and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| saveCurrentPreset | catalogued | Presets and snapshots | pending W150 |
| saveCurrentPresetAs | catalogued | Presets and snapshots | source-backed payload; execution/readback pending W150 |
| setSelectedPedalboardPlugin | catalogued | Pedalboard and parameter inspector | pending W150 |
| savePluginPresetAs | catalogued | Presets and snapshots | source-backed payload; execution/readback pending W150 |
| getPresets | catalogued | Presets and snapshots | bounded readback implemented; full browser/installed qualification pending W150 |
| setPedalboardItemEnable | catalogued | Pedalboard and parameter inspector | pending W150 |
| setPedalboardItemUseModUi | catalogued | Pedalboard and parameter inspector | pending W150 |
| updateCurrentPedalboard | catalogued | Pedalboard and parameter inspector | pending W150 |
| setSnapshot | catalogued | Presets and snapshots | pending W150 |
| setSnapshots | catalogued | Presets and snapshots | pending W150 |
| currentPedalboard | catalogued | Pedalboard and parameter inspector | bounded readback decoder and adapter projection implemented; full browser/installed qualification pending W150 |
| plugins | catalogued | Pedalboard and parameter inspector | bounded identity/catalog decoder and adapter projection implemented; full browser/installed qualification pending W150 |
| pluginClasses | catalogued | Pedalboard and parameter inspector | bounded class-tree decoder and adapter validation implemented; full browser/installed qualification pending W150 |
| hello | missing | Device session/preferences | pending W150 |
| setShowStatusMonitor | missing | Monitoring and MIDI | pending W150 |
| getShowStatusMonitor | catalogued | Monitoring and MIDI | bounded startup readback and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| version | catalogued | Device session/preferences | bounded session-scoped readback and adapter projection implemented; full installed qualification pending W150 |
| loadPreset | catalogued | Presets and snapshots | pending W150 |
| updatePresets | missing | Presets and snapshots | pending W150 |
| updatePluginPresets | missing | Presets and snapshots | pending W150 |
| moveBank | missing | Presets and snapshots | pending W150 |
| shutdown | catalogued | Device system settings | pending W150 |
| restart | catalogued | Device system settings | pending W150 |
| deletePresetItems | missing | Presets and snapshots | pending W150 |
| deleteBankItem | missing | Presets and snapshots | pending W150 |
| renameBank | missing | Presets and snapshots | pending W150 |
| openBank | missing | Presets and snapshots | pending W150 |
| saveBankAs | missing | Presets and snapshots | pending W150 |
| nextBank | missing | Presets and snapshots | pending W150 |
| previousBank | missing | Presets and snapshots | pending W150 |
| nextPreset | missing | Presets and snapshots | pending W150 |
| previousPreset | missing | Presets and snapshots | pending W150 |
| renamePresetItem | missing | Presets and snapshots | pending W150 |
| copyPreset | missing | Presets and snapshots | pending W150 |
| copyPluginPreset | missing | Presets and snapshots | pending W150 |
| setPatchProperty | missing | Pedalboard and parameter inspector | pending W150 |
| setPedalboardItemTitle | catalogued | Pedalboard and parameter inspector | pending W150 |
| getPatchProperty | missing | Pedalboard and parameter inspector | pending W150 |
| monitorPort | missing | Monitoring and MIDI | pending W150 |
| unmonitorPort | missing | Monitoring and MIDI | pending W150 |
| addVuSubscription | missing | Monitoring and MIDI | pending W150 |
| removeVuSubscription | missing | Monitoring and MIDI | pending W150 |
| imageList | missing | Device assets/library | pending W150 |
| getFavorites | catalogued | Device session/preferences | bounded startup readback and snapshot projection implemented; full browser/installed qualification pending W150 |
| setFavorites | catalogued | Device session/preferences | source-backed map and event; execution/readback pending W150 |
| setUpdatePolicy | missing | Device system settings | pending W150 |
| forceUpdateCheck | missing | Device system settings | pending W150 |
| setSystemMidiBindings | catalogued | Monitoring and MIDI | pending W150 |
| getSystemMidiBindings | catalogued | Monitoring and MIDI | bounded startup/read-only query path; installed qualification pending W150 |
| requestFileList | missing | Device assets/library | pending W150 |
| requestFileList2 | missing | Device assets/library | pending W150 |
| newPreset | missing | Presets and snapshots | pending W150 |
| deleteUserFile | missing | Device assets/library | pending W150 |
| createNewSampleDirectory | missing | Device assets/library | pending W150 |
| renameFilePropertyFile | missing | Device assets/library | pending W150 |
| copyFilePropertyFile | missing | Device assets/library | pending W150 |
| getFilePropertyDirectoryTree | missing | Device assets/library | pending W150 |
| moveAudioFile | missing | Device assets/library | pending W150 |
| setOnboarding | missing | Device system settings | pending W150 |
| getWifiRegulatoryDomains | catalogued | Device system settings | bounded startup readback and adapter projection implemented; daemon/browser/installed qualification pending W150 |
| setAlsaSequencerConfiguration | missing | Device system settings | pending W150 |
| getAlsaSequencerConfiguration | missing | Device system settings | pending W150 |
| getAlsaSequencerPorts | missing | Device system settings | pending W150 |
| requestBankPresets | missing | Presets and snapshots | pending W150 |
| importPresetsFromBank | missing | Presets and snapshots | pending W150 |
| copyPresetsToBank | missing | Presets and snapshots | pending W150 |
| getChannelRouterSettings | missing | Device session/preferences | pending W150 |
| setChannelRouterSettings | missing | Device session/preferences | pending W150 |
| DownloadModelsFromTone3000 | missing | Device assets/library | pending W150 |
| cancelTone3000Download | missing | Device assets/library | pending W150 |
| pingTone3000Server | missing | Device assets/library | pending W150 |

## Concrete semantic findings

- setControl and previewControl read ControlChangedBody and call distinct model functions.
  The inspected handlers do not issue a direct Reply. W150 must identify the actual notification
  path and correlation rules; awaiting a fabricated per-request ACK would hang.
- setSnapshot reads a scalar int64 index; setSnapshots reads SetSnapshotsBody.
  They cannot share a generic object payload without a verified adapter conversion.
- setInputVolume and setOutputVolume read scalar float values. Range and unit must come
  from the model/client metadata, not a blanket normalized 0–1 browser input.
- saveCurrentPreset supplies the server client ID to the model, with no direct Reply in this
  handler. Save-as uses a structured payload and returns a preset identity.
- saveCurrentPresetAs and savePluginPresetAs reply with the observed name
  saveCurrentPresetsAs. Preserve protocol spelling in version-specific fixtures.
- Selecting a plugin changes UI context; enabling it changes audio behavior. Distinguish those
  operation semantics in the visual editor and undo/persistence policy.
- A source-derived comparison of the 104 server handlers with the matching TypeScript client
  finds 103 client request/send names, 102 exact-name overlaps, two server registrations emitted
  through dynamic helpers (`setControl`, `previewControl`), and one case-mismatched client name
  (`downloadModelsFromTone3000` versus server `DownloadModelsFromTone3000`). This is an exact
  coverage checkpoint for the pinned revisions, not browser acceptance. Server-only registrations
  still require classification as internal, obsolete, or another-client operations; absence from
  this client is not evidence of product unsupport.
- Replies are correlated by numeric `reply` ID; textual reply names are not validated. Current
  source contains `saveCurrentPresetsAs`, `setJackserverSettings`,
  `GetFilePropertydirectoryTree`, and a `getKnownWifiNetworks` reply named `getWifiChannels`.
  Preserve these in versioned fixtures, but do not treat mismatches as general aliases.
- The matching client invokes a reply handler twice and retains its reservation until reconnect.
  MACKES must dispatch a correlation exactly once and release it on reply, timeout, cancellation,
  or disconnect.
- The web control now labels `pipedal-value` as a native control-domain value, removes the false
  `0..1` constraint, and the IPC field documents the same native-domain contract. Resolution
  entries now carry plugin URI and symbol, allowing the editor to join each persisted mapping to
  catalog `min_value`/`max_value`; the adapter remains the authoritative validator. A browser
  The deterministic fixture `docs/fixtures/pipedal-native-range-example.json` and
  `scripts/check-pipedal-range-fixture.py` cover a `-12..12` gain range; interactive browser
  observation remains separate acceptance evidence.

## Requirements to close the newly discovered gap

W150 must reconcile every row against client serializers, server body types, model side effects,
reply/event emission and matching-version fixtures. Extend the connector for user-addressable
operations missing from its catalog. Record internal helper operations with the workflow that uses
them; do not classify the entire missing catalog as unsupported. Keep service changes, file
deletion, imports, downloads and preset writes explicit user actions. No such action was run
during this read-only audit.

For each row add payload schema, response/event, value domain, persistence, confirmation and
idempotency rules, canonical editor/control and browser evidence. Test subscriptions with
unsubscribe/reconnect and bounded queues. Test library operations with non-production fixtures
and path validation. Qualify the inventoried HTTP upload/download routes before declaring the
device inventory exhaustive. External account workflows require actual account authorization at execution.
