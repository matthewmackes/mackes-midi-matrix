# ADR-0006: Profile-owned effect compatibility

## Decision

Destination choices are derived from explicit profile facts. A known effect
family produces one stable, signal-ordered block; profiles without a
trustworthy family produce a `General` block. Parameters retain exact profile
labels, bounded ranges, source-role compatibility, support state, and evidence
level. Labels are never parsed to infer ownership.

Compatibility results are renderer-neutral and explain whether a choice is
compatible, disconnected, source-role incompatible, read-only, or experimental.
Experimental entries remain visible for review but are not actionable without
the existing unsafe authorization policy.

## Compatibility

New metadata is optional when deserializing existing profiles. Existing
profiles therefore remain loadable and deterministically project into General
or their declared effect family without changing wire messages.
