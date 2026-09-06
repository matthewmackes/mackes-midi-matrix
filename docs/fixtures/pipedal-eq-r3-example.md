# PiPedal EQ R3 fixture

This fixture assigns Launch Control XL row 3 knobs to native controls advertised by the active
EQ. R3C4 uses the cross-family `gain` control when present; R3C5 through R3C8 are optional
metadata-selected controls. URI and symbols are reusable identities; runtime instance IDs must
be resolved from the current PiPedal catalog.

If more than one matching EQ instance is discovered, add an explicit `scope` before activation.
The connector fails closed on an ambiguous URI so a knob cannot change the wrong EQ instance.
`gain` is the plugin output gain. Band-level symbols are only valid when advertised by the
active EQ; three-band and parametric EQs are not forced into the same five-symbol layout.
