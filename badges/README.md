# Badge Endpoints

This directory contains generated Shields endpoint JSON for public, repo-scoped
verification badges.

Generate or refresh:

```bash
cargo run -p xtask -- badges
```

Verify committed endpoints:

```bash
cargo run -p xtask -- badges --check
```

These files summarize repository state only. Do not copy PR evidence from
`target/ripr/pr/`, review guidance from `target/ripr/review/`, or impacted
evidence from `target/xtask/impacted-evidence/` into badge endpoints.

