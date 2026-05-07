# Coverage

Coverage is execution-surface evidence.

It answers:

> Did tests execute this Rust HL7v2 parser / validation / transport / service surface?

It does not answer:

- whether HL7 conformance behavior is correct,
- whether clinical safety is proven,
- whether parser behavior is complete,
- whether profile validation is complete,
- whether MLLP/network behavior is robust,
- whether HTTP/gRPC contract behavior is complete,
- whether Python bindings are validated,
- whether mutation adequacy is strong,
- whether fuzzing is sufficient,
- whether publish readiness is proven.

Those are separate proof lanes.

The Coverage workflow runs on:

- push to `main`,
- `workflow_dispatch`,
- PRs labeled `coverage` or `full-ci`.

Codecov comments are disabled. Durable receipts are:

- `coverage.json`,
- `coverage.txt`,
- `lcov.info`,
- the GitHub Actions coverage artifact,
- the Codecov dashboard.
