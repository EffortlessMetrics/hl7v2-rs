# hl7v2-python

## Build
```bash
cargo build -p hl7v2-python
```

## Test
```bash
cargo test -p hl7v2-python
```

## Lint
```bash
cargo clippy -p hl7v2-python -- -D warnings
```

## Python Wheel Build (with maturin)
```bash
cd crates/hl7v2-python
maturin develop  # For development
maturin build --release  # For release wheels
```
