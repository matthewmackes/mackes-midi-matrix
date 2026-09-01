# MACKES MIDI Matrix 0.1.11

## First-time operator walkthrough

1. Run `mackes-midi-matrixd`, then launch `mackes-midi-matrix`.
2. Use the five task sections: Live, Map Controls, Scenes, Devices, and System.
3. In Map Controls, press Device, move exactly one eligible Launch Control XL Mk2
   knob, button, or fader, then choose the device, effect, and parameter with the
   controller arrows. Press Device again to commit.
4. Select and verify Factory Template 1 on the Launch Control XL Mk2 using the
   [Factory 1 contract](mackes-launch-control-xl-mk2-factory1-manifest.json).
5. If the template is missing or mismatched, follow the inline recovery message;
   MACKES will not guess an input assignment.

Existing projects and mappings are migrated conservatively. Ambiguous legacy
Launch Control fader records remain inactive until recaptured. The compatibility
route for low-level legacy capabilities is under System → Advanced → Legacy while
the parity work is completed.

## Rollback

Keep the previous configuration and reviewed Components artifact before upgrading.
To roll back, stop the daemon, restore the prior configuration backup, reinstall
the previous package, and reconnect the controller. Do not switch away from User 1
or overwrite the factory templates during recovery.
