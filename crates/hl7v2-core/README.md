# hl7v2-core

Deprecated compatibility facade for hl7v2-rs.

Use the `hl7v2` crate for new Rust code:

```toml
[dependencies]
hl7v2 = "1.2.0"
```

`hl7v2-core` is retained temporarily while the old implementation microcrates
are collapsed into modules under `hl7v2`. It should not be treated as a second
public facade or a stable product surface.

## Usage

For usage examples, see the
[examples/](https://github.com/EffortlessMetrics/hl7v2-rs/tree/main/examples)
directory in the root of the repository. Those examples import through
`hl7v2`.
