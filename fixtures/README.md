# Fixtures

Only redacted, legally redistributable protocol fixtures belong here. Do not commit vendor PDFs,
hardware backups, USB serials, usernames, absolute paths, credentials, or private MIDI captures.
Use `fixtures/local/` for private captures; that path is ignored by Git.

`config-valid.json5` also demonstrates the persisted dashboard MIDI binding
format. Bindings must be explicit and are limited to `panic`, `next_scene`, and
`previous_scene`; invalid or duplicate triggers are rejected during validation.

`config-scenes-valid.json5` is a redacted two-scene project fixture for testing
scene navigation, active-scene persistence, and safe scene planning.
