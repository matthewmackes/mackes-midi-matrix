# Retired: MACKES User 1 — Launch Control XL Mk2

This document is retained only as migration history. It is not an install
procedure. Production uses
[Factory Template 1](mackes-launch-control-xl-mk2-factory1-manifest.json):
operators select Factory Template 1 on the controller. MACKES does not install,
send, or claim a Novation Components User template.

The former Components User 1 workflow, User 1 selection SysEx, and nullable
artifact checksum are retired. Legacy mappings with User 1 tuples are migrated
transactionally by `migrated_factory1`; ambiguous records fail closed and must
be recaptured.
