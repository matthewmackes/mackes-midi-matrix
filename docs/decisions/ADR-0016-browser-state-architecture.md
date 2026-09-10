# ADR-0016: Same-origin browser state architecture

**Status:** Accepted for incremental migration
**Date:** 2026-09-08

## Decision

MACKES keeps a dependency-free browser application served from the same origin as the web API.
The current shell remains plain HTML, CSS, and JavaScript so the appliance works offline and does
not require a package registry, CDN, runtime transpiler, or third-party license bundle. Migration
to bounded components is incremental: one explicit lifecycle boundary owns navigation, request
cancellation, generation ordering, draft protection, and health state; feature renderers remain
small and may be extracted behind that boundary without changing API ownership.

Read-only workspace loads use a per-navigation `AbortController` and a monotonic generation. A
superseded request is canceled, and a response that nevertheless arrives is ignored when its
generation is stale. Background refresh skips dirty forms and route drafts. Returning to a visible
tab refreshes health and the active workspace immediately. Mutations remain daemon-owned and are
never cached by the browser read path.

## Provenance and constraints

The implementation uses browser-standard `fetch`, `AbortController`, `visibilitychange`, History
API, and SVG/DOM semantics. No external runtime dependency is introduced. The governing web API,
capability, and same-origin requirements are recorded in `docs/web-api-v1.md`,
`schemas/capability-boundary.schema.json`, and W159/W163 in `WORKLIST.md`.

The extracted navigation module is pinned at SHA-256
`583d5e9c6e970a7db5534ba54c435b64365cef15cf1b928e9f25ecdeb18b1b33`
(`apps/mackes-web/static/navigation.js`, measured 2026-09-08).
The extracted health policy module is pinned at SHA-256
`e2464619ec7c67d637a101b0da4d864a2a118d940da8dabc6032d0dd4c6df3df`
(`apps/mackes-web/static/health.js`, measured 2026-09-08).

The researched product-feature catalog is likewise served as a pinned same-origin module
(`apps/mackes-web/static/feature_catalog.js`), and browser qualification hashes it alongside the
state modules so catalog drift cannot be hidden by a matching application bundle.
The qualified source hash is
`16152e3037413b2c0719baca295606d1f3f641220a058a21fddb80ec96bdd8f1`.

The pure feature-search renderer is pinned at SHA-256
`7847f57fc92229f8de55c1952b3e0e798dd1a6bd3eb04708f90fd3e272e8d8b9`
(`apps/mackes-web/static/feature_renderer.js`, measured 2026-09-08). It owns only searchable text
and filtering projection; transport, mutation, and authoritative state remain in the application
and daemon boundaries.

## Evidence boundary

`scripts/browser-generation-smoke.py`, `browser-abort-smoke.py`,
`browser-resume-smoke.py`, and `browser-draft-preservation-smoke.py` provide deterministic live
fixtures for ordering, cancellation, hidden-tab recovery, and draft preservation. These fixtures
do not claim native hardware readback. Full component extraction, focus/scroll preservation across
every feature renderer, and W165's daemon-owned modifier contract remain open work.

## Consequence

The project can improve state isolation without introducing a new build/provenance surface. The
remaining migration work must preserve the no-CDN boundary, record any new source/license hashes,
and keep the daemon as the authority for mappings, scenes, and hardware state.

## Next migration boundary

Navigation and feature projection are extracted components. Health polling remains coupled to the
shared DOM and failure counters until a pure status model can be extracted with deterministic tests;
the feature renderer does not split authority because it is a read-only projection.

The shared browser state store boundary is pinned at SHA-256
`a197eea9306cf133ab636fa3a92ba62d75b99dc7b5cfec77143dcd6fb3f4e742`
(`apps/mackes-web/static/state_store.js`, measured 2026-09-08). It rejects stale generations,
publishes immutable snapshots, and exposes unsubscribe handles for incremental renderer migration.
