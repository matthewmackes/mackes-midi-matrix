# Contributing

Work is governed by `WORKLIST.md`. Claim one `READY` item, record the owner and start date, and
do not alter a public contract without an ADR and compatibility tests.

Keep vendor manuals, private captures, credentials, machine-specific paths, and generated output
out of the repository. Hardware-writing tests must be ignored by default and require explicit
arming and a device/port argument.

Record discovered facts in the owning audit or inventory document as soon as they are established.
Every source record needs its authority and URL/path, revision or commit, retrieval date,
SHA-256 (or another immutable identifier), exact page/section/symbol, evidence class, and known
limits. Use manufacturer/vendor documentation, developer documentation, and matching source
before binary inspection or reverse engineering. Link later work to the existing record so the
same fact is not rediscovered or reinterpreted without a new evidence entry.
