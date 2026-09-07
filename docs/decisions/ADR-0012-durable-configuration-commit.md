# ADR-0012: Durable configuration commit boundary

## Decision

Configuration writes use one validated document as the commit unit. The writer validates the
complete document, creates a uniquely named temporary file in the target directory, writes the
full serialized document, calls `sync_all` on that file, atomically renames it over the target,
and calls `sync_all` on the parent directory. Prior generations are rotated as numbered backups;
backup rotation also synchronizes the parent directory.

The in-memory daemon state is changed only after the durable writer succeeds. A failed validation,
backup rotation, temporary-file write, sync, or rename is reported as a failed operation and must
not be acknowledged as saved. Unknown or ambiguous endpoint references remain unchanged for
operator repair.

Daemon-owned route state and its undo snapshot use a bounded same-directory prepare journal
(`*.routes.commit.json`). The journal contains complete replacement documents, is synchronized
before either file is replaced, and is removed only after both files and their parent directory
are synchronized. Startup recovery replays an incomplete journal before loading route state;
malformed, oversized, or path-escaping journals fail closed and remain for diagnosis.

## Consequences

This contract makes restart recovery select a complete old or new configuration document rather
than a partially written file. Backup files are recoverable and validated before restore. Other
independent files still require their own coordinated protocol.

## Verification

`crates/config` tests cover atomic save, backup rotation, migration dry-run/apply, ambiguous-plan
abort, backup failure, and preservation of the original document. The route/undo journal has
simulated interrupted-recovery coverage; physical power-loss and disk-full qualification remain
external evidence.
