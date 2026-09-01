# Retired: MACKES User 1 — Launch Control XL Mk2

This document is retained only for migration history. The production contract is
[Factory Template 1](mackes-launch-control-xl-mk2-factory1-manifest.json).

This is the reviewed setup contract for the second-generation Novation Launch Control XL.
It is intentionally a Components workflow: MACKES does not write template definitions or
send undocumented template SysEx.

## Install

1. Open the official Novation Components application and connect a Launch Control XL Mk2.
2. Select the target device and choose User 1.
3. Enter the assignments from the checked-in [manifest](mackes-launch-control-xl-mk2-user1-manifest.json).
4. Send the template to the device using Components.
5. Save/export the reviewed Components artifact and record its SHA-256 in the manifest.
6. Reconnect the controller and verify that all 24 knobs, 16 channel buttons, and 8 faders
   produce the expected unique inputs. Device, Mute, Solo, Record Arm, Up, Down, Left, and
   Right remain reserved interface controls.

## Verification and recovery

MACKES selects User 1 on initial connection and reconnect, compares observed eligible inputs
with the manifest, and restores authoritative base LEDs. It does not switch templates on
ordinary TUI exit.

If the model, generation, User slot, checksum, or observed input inventory disagrees, the
interface shows `MACKES TEMPLATE REQUIRED` and does not guess assignments. Reopen Components,
select the correct Mk2 device and User 1, resend the reviewed artifact, then reconnect. If
the artifact is missing or corrupt, restore the last reviewed artifact and its manifest
checksum; no device-template state is overwritten by MACKES.

## Update and rollback

Any template revision requires a new version, reviewed inventory, target-model declaration,
artifact checksum, and a retained prior artifact for rollback. Physical appearance and MIDI
qualification remain separate hardware checks.
