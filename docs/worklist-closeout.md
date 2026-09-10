# Worklist closeout

This execution index reorganizes all 37 currently unchecked work items without
discarding scope. It separates delivery from parent closeout. Existing item status
records describe accumulated work; they do not imply 35 simultaneous active jobs.
Execute one bounded implementation checkpoint at a time. No new operator approval
is required for routine implementation already covered by the accepted worklist.

## Drain order

| Order | Delivery owner | Related scope to reconcile | Remaining exit condition |
| --- | --- | --- | --- |
| 1 | W153 | — | **DONE (2026-09-07).** Complete model/schema field inventory and valid/invalid non-default fixtures. `docs/configuration-field-catalog.md` and the complete round-trip/invalid fixture tests are the evidence. |
| 2 | W154 | Configuration portion of W146 | Freeze draft, validation, diff, apply, operation, compatibility and normalization contracts. **Next claim.** |
| 3 | W155 | Persistence portion of W139 | Capture revisions at the original read; migrate supported writers; durable drafts, idempotency and restart recovery. |
| 4 | W156 | Runtime portion of W139 | Preserve complete reads; prepare runtime consumers; report persisted/runtime divergence and restart requirements. |
| 5 | W157, then W158 | Configuration portion of W151 | Complete shared form/JSON5 draft editing and integrated edit/apply/restart/read scenarios. |
| 6 | W145, remaining W146 | — | Pin missing technical facts once; resolve remaining device capability and draft contracts. |
| 7 | W147 | W133 | Finish shared responsive controls and accessibility once for all editors. |
| 8 | W148 | W099, W110, W122, W127, W128, W135 | Complete Novation assignment, binding/recovery and PiPedal ownership behavior; reconcile each older acceptance clause. |
| 9 | W149 | Device portion of W138 | Finish Eventide and Reflex editors against sourced capabilities. |
| 10 | W150 | W114, remaining W138 | Finish PiPedal mapping persistence, operator workflows and exhaustive workspace. |
| 11 | Remaining W151 | W136, W137, remaining W139 | Finish routes, transformations, endpoints/network, projects/scenes/setlists and non-config management scope. |
| 12 | W140 | — | Integrate operation lifecycle, concurrency and resynchronization across completed mutation families. |
| 13 | W152 | W100, W102, W104, W116, W126, W132 | Reconcile boot/readiness/install requirements, verify the final artifact, deploy under existing authorization and verify served build identity. |
| 14 | Parent acceptance audit | W111, W117, W129, W144 | Close only after all associated delivery and residual acceptance clauses are accounted for. |

Rows are scheduling groups, not replacement dependency declarations. Within each
group follow the original prerequisites. If a prerequisite is incomplete, work on
it first; do not label normal unfinished implementation an external blocker.
Older parent requirements not implemented by the named delivery owner remain
explicit residual work in that original item. W139 also retains SysEx, profiles
and backup requirements beyond the new configuration children.

Dependency audit: all 37 open IDs are represented above. W111 cannot close with
the PiPedal editor because it also depends on W116 deployment. W117 cannot close
with the Novation editor because it also depends on W126 deployment. Both are
therefore explicitly scheduled for the final parent audit. In group 8, finish
W122 and W127 before W128. In group 13, finish W100 and W102 before W104 (and
confirm W099 from group 8); finish W114 before W116. W152 waits for all four
editor owners W148–W151. These are existing dependencies, not new scope.

The configuration HTTP service in group 5 can precede the shared visual shell;
its editor integration and W157/W158 closure wait for the reusable controls from
group 7 where needed. W154 may establish the configuration contract increment
before the broader W146 closes; it must reuse existing envelope conventions.
Do not duplicate those controls or envelopes to force a strictly linear schedule.

Next claim: W154's remaining versioned configuration and durable draft contracts. The immediate repair
list below is recorded under its existing W155/W156 owners and must be resolved
before their acceptance, alongside their other remaining requirements. After each
claim, identify the exact residual acceptance clause being implemented, finish it,
and select the next eligible prerequisite. Keep only one current execution claim;
partially implemented downstream items retain their historical progress records.

## Immediate repair checkpoint

Source inspection establishes these defects and limits before further expansion:

1. W156: resolved 2026-09-07. `configuration_response::encode` now preserves the
   catalog's full `configuration` document and emits a separate
   `configuration_available` marker; `encode_preserves_configuration_projection`
   covers the serialized read response.
2. W155: resolved 2026-09-07. `startup_restore.rs::persist_active_scene` now
   captures the original document revision and passes it directly to
   `save_if_revision`, eliminating the second-read lost-update window. Startup
   restore tests pass; an injected interleaving writer test remains part of the
   broader recovery acceptance.
3. W155: the CLI/import inventory is a starting point, not proof of full migration.
   Trace internal config-store, restore and migration APIs as well as direct
   application save calls. Operator-supplied tokens are not necessary for an
   internal read/modify/write transaction to retain its original revision.
4. W156: resolved 2026-09-07. Control-store conversion now occurs before the
   atomic save; invalid runtime mappings are rejected before persistence, and the
   prepared store is installed only after the save succeeds. The daemon suite
   covers the resulting mutation path.

These source observations supersede earlier broad claims of complete reads and
conflict-safe startup persistence. They require implementation, not a new scope
decision. Source authority: local symbols above, inspected 2026-09-07; evidence
class: code inspection of the current uncommitted tree, not runtime qualification.

## Closure rules

- Preserve all existing acceptance criteria. Record a short residual checklist
  when a delivery group finishes; close older overlapping items only when their
  additional requirements are satisfied.
- W134, W141, W142 and W143 are already checked. Reuse their evidence for the
  scope it covers. Their DONE status does not prove the newer WYSIWYG scope or
  an untested current tree is complete. Reconcile their unfinished prerequisite
  references during final integration rather than silently propagating DONE.
- Keep physical qualification under the existing operator-approved post-release
  treatment. Never label assumptions as measurements. Removed hardware stays out
  of scope.
- Use focused verification for each implementation change. Run the required full
  gate at integration/release checkpoints; record the revision or worktree
  identity tested. Do not repeat stale green-gate claims after subsequent changes.
- Every continuation must implement a residual requirement, resolve a concrete
  unknown, or poll a confirmed live job. Status-only continuations do not drain
  this queue. A code defect or module size limit is actionable engineering work.
- Final closure requires a requirement-by-requirement pass over the original
  items and a verified installed artifact for deployment scope; a green build
  alone is insufficient.
# Project drain standard

Every worklist execution follows this standard objective:

> Drain the worklist decisively: eliminate every unjustified `IN_PROGRESS` item, prioritize the
> highest-impact vertical slice, implement missing functionality immediately, close each item only
> with direct evidence, and run the full release gate after every major slice. Surface blockers
> within one cycle, document authoritative sources and limitations, and leave no stale, duplicate,
> or qualification-dependent work hidden in the backlog.

This statement is the project-wide definition of drain. A task may be closed only when its
acceptance evidence is recorded in `WORKLIST.md`; external qualification dependencies must remain
explicit rather than being implied by green software tests.

### Default drain-goal execution policy

For every project drain goal, execute every open item and child task sequentially. Broad scope,
unfinished dependencies, long duration, or missing qualification evidence are reasons to decompose
the work into the next bounded implementation slice—not reasons to stop. Each slice must be
recorded in the worklist, implemented, tested, and followed immediately by the next available
slice. Fan out independent slices whenever possible, coordinate their contracts, and work
aggressively toward the goal without waiting unnecessarily for sequential completion. Never
substitute status reporting for execution. Mark the goal blocked only when a concrete
external dependency or required user decision prevents any safe next action for three consecutive
attempts. Completion requires every programmatic acceptance criterion to be implemented and
verified; qualification-only evidence remains explicitly tracked and must not block software work.
If a token-usage warning occurs, immediately document the current state and work performed in the
worklist, save all changes and evidence, then stop cleanly without starting another slice.

## Device HUD decision record (2026-09-07)

Operator-confirmed requirements for the reusable per-device HUD pattern: support an exact/generic
layout toggle only when the profile advertises both; use a persistent right-side inspector that
collapses below the stage on narrow screens; prioritize destination and assignment lifecycle through
a searchable compatible catalog; preview before explicit Apply; expose draft/pending/applied/
acknowledged/observed/error states; keep single-control selection active after apply; and show
unsupported features visibly disabled with their source/reason. These decisions are requirements for
W147/W148 and must not be treated as browser qualification evidence.

The implemented checkpoint now includes the profile-gated exact/generic toggle, persistent responsive
inspector, searchable bounded assignment catalog, preview-before-apply mapping flow, lifecycle status
projection, and disabled unavailable-feature presentation. These are software evidence only; native
physical behavior and browser visual acceptance remain separate qualification activities.
