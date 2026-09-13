# MOD artwork source inventory

Recorded 2026-09-13 for W204. The operator confirmed that assets approved in the remote repository
are fully authorized for this private project. The manifest permission-basis value is
`operator-confirmed-remote-repository-approval-2026-09-13`.

## Pinned source

- Project: `mod-audio/mod-ui`
- Canonical repository: <https://github.com/mod-audio/mod-ui>
- Revision: `c3004836e3466fa3a0d34de3ca855530e75304d1`
- Repository license file: `LICENSE` (GNU AGPL version 3)
- Intended use: static visual assets only; no upstream runtime, protocol, persistence, store, social,
  account, telemetry, or network-service code.

The operator's authorization is the permission basis for this private integration. The upstream
license and source notice must still ship beside imported files. Hashes below are SHA-256 values for
the unmodified upstream files and become `upstreamSha256` in the vendor manifest. A separately
computed `vendoredSha256` records any approved optimization or sanitization.

## Accepted candidate tranche

| Role | Upstream path | Dimensions | Upstream SHA-256 |
|---|---|---:|---|
| audio input port | `html/img/audio-input.png` | 88×56 | `47455729e2790d070cc9c43a7aabb43103478568b0f42e32dd7a536bf51e3b67` |
| audio output port | `html/img/audio-output.png` | 87×56 | `a5a613346cf47ae8a46e9c6ca6ea228c4d4ecc39f1f54f1d74b58a05c8772103` |
| audio connected jack | `html/img/audio-jack.png` | 128×56 | `2fd61cd24709391464cace399874ad79eb7579b7b8399b0c90ced7709af550f2` |
| audio open jack | `html/img/audio-jack-nconnect.png` | 87×56 | `2213029f18dddc5711e1670d2c08028584bbba776a076e3208bba6a1a3d82cb1` |
| CV input port | `html/img/cv-input.png` | 80×56 | `fe5f3098e180f2da52e39d070d7a1c45164d6fbcb14080b3727beeb6f4286aab` |
| CV output port | `html/img/cv-output.png` | 80×56 | `dfea50a73751fae258ba72813604005350b008cc70b0b019ab396905e55e878e` |
| CV connected jack | `html/img/cv-jack.png` | 87×56 | `d2326b93c4642866b8013503c6ab2340956a93b805636d55c8a829e9c462f0ce` |
| CV open jack | `html/img/cv-jack-nconnect.png` | 80×56 | `4a6629f42bdc0c7bf7460d37478f661295066dedf676dc4a71aab5e6e6034c1e` |
| MIDI input port | `html/img/midi-input.png` | 76×56 | `565c3337e8d237ab9c522ea0806cdbfa0a4e2cd345bcc390a77e9d171a228732` |
| MIDI output port | `html/img/midi-output.png` | 78×56 | `5f7cce33f3309a980661cc065c1105c68f02ecb20e810b2c1f6f0ee29ead933d` |
| MIDI connected jack | `html/img/midi-jack.png` | 78×56 | `b5dacbf85f6df704be4b575b0113ae16fd9eaa68d81f43ce5f077a47a11058b0` |
| MIDI open jack | `html/img/midi-jack-nconnect.png` | 78×56 | `354810905d653f77141044a33d00f5e664b0417f037164c9db101c821ee2afe8` |
| MIDI plug alias | `html/img/midi-plug.png` | 78×56 | `b5dacbf85f6df704be4b575b0113ae16fd9eaa68d81f43ce5f077a47a11058b0` |
| footswitch strip | `html/img/footswitch.png` | 176×88 | `bfdc41417cb363892ae1e67563a82aa00b596435e1209ee077a6177215f73bf4` |
| assigned footswitch marker | `html/img/footswitch-addressed.png` | 16×16 | `87efc68785af3344f4120231f4ca08fa24518aa65bada438fda69f28607cf129` |
| free footswitch marker | `html/img/footswitch-free.png` | 16×16 | `2a502c96a7cdeca7535b90412d9ebd30e9ad6a02790297defe62ff9e3d56f4de` |
| rotary sprite | `html/img/knob.png` | 8320×128 | `0f875a8bca79b222bdaeb24c15ec3a6deae52f36d24683440aec477d4e1d5db2` |
| slider sprite | `html/img/slider.png` | 6500×250 | `b37b0bc74f08301f02c5fbea7e120e6dacea2d9188d8fa117329e4bfb794ad98` |
| switch vector | `html/img/switch.svg` | SVG | `a7c30099fced184ef04578097c2db474152639f2b91503ec9b3a6b2afb523b66` |
| rocker off | `html/img/rocker-switch-off.png` | 52×62 | `53a21d09b4e55a629bd0d234bd73dc6f6080dfc20258ae1ddaa5a7f357bafbc2` |
| rocker on | `html/img/rocker-switch-on.png` | 52×62 | `d42895a5e523771a33fdfb2e546a7e3d609fb8ef30938019c97c814a26428e53` |
| red LED off | `html/img/red-light-off.png` | 32×32 | `37010d78941a78e213c092a9be0022c15af080b27911b842b0fdd1990a99a4ba` |
| red LED on | `html/img/red-light-on.png` | 32×32 | `9bedded18815070b534a1e146cd6393b150109e4b41036bb42edd75d4b5479da` |
| purple LED off | `html/img/purple-light-off.png` | 32×32 | `e6fc4b461e24caef02283c66a32ac2742e08aa71e29ed5720ce9154183c5d3e3` |
| purple LED on | `html/img/purple-light-on.png` | 32×32 | `fd052d1aca64e089b76e7c6e8dbe37bcc07b9813b0a8bdafe8486093d397e9a7` |
| rack surface | `html/img/rack.png` | 1024×1024 | `6d80a667c215169afc014947bf208ad597b7a319c5fef5b1b025f69b41ce369c` |
| library surface | `html/img/library-background.png` | 1024×100 | `4e212ba69d76aea1f7e2de67d35806d9c4135cd57df8430f65ef07836f67b5c2` |
| blocked state | `html/img/icons/blocked.svg` | SVG | `2e9c5ba1a9448328f68aeed069982c5dbe2a2fe85aa7cac8ea018a389390aba4` |
| missing-art state | `html/img/icons/broken_image.svg` | SVG | `30bd597b2fd879f513b2d7f9de788683c2c9a484b2188cb7f9777b7e969bc46a` |
| broken-plugin state | `html/img/icons/broken_pedal.svg` | SVG | `63d8cd3dca50fddcc614341f6d041ce5a5b2dc8d8c51910be33142198f63c28d` |
| transport state | `html/img/icons/transport.svg` | SVG | `9f8ae6b3daf99c32e54c6fcb6b2c001375ce0de5a762a5631aa22b6e84521acf` |

`midi-plug.png` is byte-identical to `midi-jack.png`; vendor one canonical file and represent the
second role as a manifest alias unless visual review establishes a semantic difference.

## Excluded classes

- MOD logos, favicons, social-network images, avatars, watermarks, and product branding;
- cloud/store, ratings, sharing, account, update-marketing, and web-service artwork;
- screenshots and default-pedalboard thumbnails, which are references rather than primitives;
- third-party plugin branding until its own source path, creator, permission, and role are recorded;
- fonts and icon fonts; MACKES keeps its existing offline typography and accessible text labels;
- loading GIFs and ornamental navigation chrome that conflict with reduced motion or the Studio
  shell;
- any JavaScript, Python, CSS behavior, protocol code, persistence behavior, or remote dependency.

## Remaining W204 work

1. Create the vendor directory, notice, manifest schema, and deterministic validator.
2. Import the accepted files, sanitize the four SVG candidates, and compute vendored hashes.
3. Review each rendered asset for branding or embedded text missed by filename inspection.
4. Add orphan, duplicate, MIME, dimensions, external-reference, script, and hash failure fixtures.
5. Record final accept/reject disposition and only then mark W204 complete.
