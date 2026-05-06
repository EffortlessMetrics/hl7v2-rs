# hl7v2-path

Deprecated compatibility crate for HL7 v2 path parsing.

New Rust code should depend on `hl7v2` and use either:

```rust
use hl7v2::{Path, parse_path};
```

or:

```rust
use hl7v2::query::path::{Path, parse_path};
```

This crate is retained temporarily while the old implementation microcrates
collapse into modules under `hl7v2`. It re-exports the `hl7v2::query::path`
API and should not gain new behavior.

## Usage

For usage examples, see the [examples/](https://github.com/EffortlessMetrics/hl7v2-rs/tree/main/examples) directory in the root of the repository.
