# PiPedal EQ R3 fixture

This fixture assigns Launch Control XL row 3 knobs R3C4 through R3C8 to the qualified
`toob-parametric-eq` controls. The URI and symbols are reusable identities; runtime instance IDs
must be resolved from the current PiPedal catalog.

If more than one matching EQ instance is discovered, add an explicit `scope` before activation.
The connector fails closed on an ambiguous URI so a knob cannot change the wrong EQ instance.
`gain` is the plugin output gain; the other four symbols are the low, low-mid, high-mid, and
high band levels.
