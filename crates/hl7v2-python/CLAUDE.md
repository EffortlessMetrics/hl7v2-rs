# hl7v2-python

PyO3 binding backend crate for the public Python `hl7v2` package.

This crate is language-boundary infrastructure. It is publishable on crates.io
for packaging provenance, but it is not the recommended Rust API. Rust users
should depend on `hl7v2`; Python users should install the public Python
distribution named `hl7v2`.

## Build And Proof

```bash
cargo +1.95.0 run -p xtask -- python-local-wheel-proof
```

The local wheel proof builds the Python `hl7v2` wheel, installs it into a
scratch virtual environment, imports `hl7v2`, and runs the Python smoke and
evidence workflow scripts. It does not claim TestPyPI or PyPI availability.

## Public Registry Boundary

TestPyPI and PyPI proof must use the public package name `hl7v2`:

```bash
cargo +1.95.0 run -p xtask -- python-public-registry-proof --index testpypi --version <version>
cargo +1.95.0 run -p xtask -- python-public-registry-proof --index pypi --version <version>
```

Do not add token fallback, do not use `skip-existing`, and do not claim
TestPyPI or PyPI success without upload and install-back proof.

## Role

Keep this crate thin over `hl7v2`. Its job is Python ABI, conversion, lifecycle,
and packaging proof for the Python user surface. Do not move parser, model,
redaction, MLLP, batch, or stream implementation logic here.

When changing Python-exposed evidence behavior, update the shared Rust behavior
or explicit conversion layer, then update Python smoke, evidence parity, and
public-registry proof docs as needed.
