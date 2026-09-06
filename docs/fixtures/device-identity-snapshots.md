# Durable device identity snapshots

These redacted snapshots exercise the W105 identity contract. Runtime ALSA
addresses differ between observations; durable aliases and logical port
identities do not change.

## Before reconnect

```text
alias=midisport-4-in
vendor=0763 product=1021 serial=<none>
logical_port=3 direction=input role=midi
runtime_address=24:3
state=connected
```

## After reconnect at a different ALSA address

```text
alias=midisport-4-in
vendor=0763 product=1021 serial=<none>
logical_port=3 direction=input role=midi
runtime_address=130:0
state=connected
```

The serial-less unit resolves only because its persisted operator binding is
explicit. A second matching candidate is ambiguous until the operator selects
a binding. Values are synthetic and contain no captured serials or private
host paths.
