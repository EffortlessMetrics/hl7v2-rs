# AGPL Publish-Readiness Licensing Audit

**Repository:** EffortlessMetrics/hl7v2-rs
**Date:** 2026-03-06
**Auditor:** Automated scan + manual review
**License:** AGPL-3.0-or-later
**Verdict:** **GO for publishing**

---

## Executive Summary

All 30 Cargo.toml packages, the LICENSE file, README, CHANGELOG, CLA, CONTRIBUTING, OpenAPI spec, GitHub release notes, and CI enforcement are uniformly AGPL-3.0-or-later. No stale self-licensing claims exist in the current tree. The sole historical MIT/Apache reference was in the workspace Cargo.toml for exactly 2 early commits before being changed to AGPL, while the LICENSE file was AGPL from the very first commit. No history rewrite is needed.

---

## A. Current-Tree Status Table

| Surface | Status | Detail |
|---|---|---|
| `LICENSE` (root) | CLEAN | Full AGPL-3.0 text, 662 lines, present since initial commit |
| `Cargo.toml` `[workspace.package].license` | CLEAN | `"AGPL-3.0-or-later"` (line 45) |
| `Cargo.toml` `[package]` (hl7v2-examples) | CLEAN | `license.workspace = true` (line 84), `publish = false` (line 85) |
| All 28 `crates/*/Cargo.toml` | CLEAN | All use `license.workspace = true` (line 9 in each) |
| `xtask/Cargo.toml` | CLEAN | `license.workspace = true` (line 8) |
| `README.md` | CLEAN | Lines 326-335: explicitly states AGPL-3.0-or-later |
| `CHANGELOG.md` | CLEAN | Lines 238-241: states AGPL-3.0-or-later |
| `CONTRIBUTING.md` | CLEAN | Lines 9-14: states AGPL-3.0-or-later |
| `CLA.md` | CLEAN | Lines 17-24: license grant under AGPL-3.0-or-later |
| `THIRD_PARTY_NOTICES.md` | CLEAN | Documents dependency licenses (MIT, Apache-2.0, BSD, ISC, etc.) |
| `deny.toml` | INTENTIONAL | Allow-list includes MIT, Apache-2.0, etc. for *dependencies only*; AGPL-3.0-or-later included for self |
| `schemas/openapi/hl7v2-api.yaml` | CLEAN | Lines 37-39: `name: AGPL-3.0-or-later` with URL to LICENSE |
| `flake.nix` | CLEAN | No license field (relies on root LICENSE — standard for Nix flakes) |
| `.github/workflows/security.yml` | CLEAN | Lines 106-124: Active regression check blocks MIT/Apache self-claims |
| Source code (.rs files) | CLEAN | No SPDX identifiers or license headers in source files |

### Current-Tree Scan Methodology

Scan command:
```
rg -i "MIT|Apache-2\.0" --glob '!deny.toml' --glob '!Cargo.lock' --glob '!THIRD_PARTY_NOTICES.md' --glob '!target/**'
```

**Result:** All matches are false positives — words like "SMITH", "delimiter", "limitation", "commit", "submitted", "permitted" contain "MIT" as a substring. Zero actual self-licensing claims found.

Full scan output saved to: `docs/governance/licensing/receipts/current-tree-license-scan.txt`

---

## B. History Classification Table

| Commit | String Matched | Classification |
|---|---|---|
| `1aa9ab5` (initial commit) | `LICENSE` file = AGPL-3.0 | CLEAN — AGPL from day one |
| `d04e739` (2nd commit) | `Cargo.toml`: `license = "MIT OR Apache-2.0"` | HISTORICAL-ONLY — placeholder metadata in early scaffold, never published/tagged |
| `8ad8fa6` (3rd commit) | `-license = "MIT OR Apache-2.0"` -> `+license = "AGPL v3"` | HISTORICAL-ONLY — the fix commit; MIT/Apache existed for exactly 2 commits |
| Later commits | `license = "AGPL v3"` -> `"AGPL-3.0-or-later"` | CLEAN — normalized to proper SPDX identifier |
| `v1.2.0` tag | `license = "AGPL-3.0-or-later"` at tag | CLEAN |
| `deny.toml` history | MIT, Apache-2.0 in allow-list | INTENTIONAL — dependency policy, not self-licensing |
| `THIRD_PARTY_NOTICES.md` history | MIT, Apache-2.0 attributions | INTENTIONAL — compliance documentation |
| Commit messages mentioning MIT/Apache | `8ad8fa6`, governance commits | INTENTIONAL — describing the license change and enforcement |

### Key Finding

The LICENSE file has been AGPL since the initial commit (`1aa9ab5`). Only the Cargo.toml metadata field briefly said `"MIT OR Apache-2.0"` for 2 unpublished commits before being corrected in `8ad8fa6`. This is a non-issue — the actual license grant (LICENSE file) was always AGPL.

### Commit Messages Referencing Licenses

- `8ad8fa6` — "Update package license to AGPL v3 from MIT OR Apache-2.0"
- `82156fc` — "update licensing information to AGPL-3.0-or-later and add Contributor License Agreement"
- `1239653` — "catch standalone MIT in license regression check"
- `19ff46d` — "add license regression check to identify stale license references"
- `24db09f` — "Restore license governance and third-party notices"

Full history scan output saved to: `docs/governance/licensing/receipts/history-license-search.txt`

---

## C. Release Artifact Status

| Surface | Status |
|---|---|
| **GitHub Release `v1.2.0`** | CLEAN — body explicitly states "License: AGPL-3.0-or-later (see LICENSE)" |
| **Release binary assets** | None — no binary artifacts attached to the release |
| **Source availability** | Available via git tag `v1.2.0` pointing to commit `1782d9a` |
| **LICENSE included at tag** | Yes — verified `git show v1.2.0:LICENSE` = full AGPL text |
| **THIRD_PARTY_NOTICES at tag** | Yes — present at `v1.2.0` |
| **AGPL object-code obligations** | N/A — no binary artifacts distributed; source-only release |

Full release notes saved to: `docs/governance/licensing/receipts/release-notes.txt`

---

## D. Cargo.toml License Field Inventory

### Workspace Root (Cargo.toml)
```
license = "AGPL-3.0-or-later"   # line 45
```

### All Crates (28 crates + xtask + hl7v2-examples)
Every crate uses `license.workspace = true`, inheriting `AGPL-3.0-or-later`:
- `crates/hl7v2-ack/Cargo.toml` — line 9
- `crates/hl7v2-batch/Cargo.toml` — line 9
- `crates/hl7v2-bench/Cargo.toml` — line 9
- `crates/hl7v2-cli/Cargo.toml` — line 9
- `crates/hl7v2-core/Cargo.toml` — line 9
- `crates/hl7v2-corpus/Cargo.toml` — line 9
- `crates/hl7v2-datatype/Cargo.toml` — line 9
- `crates/hl7v2-datetime/Cargo.toml` — line 9
- `crates/hl7v2-e2e-tests/Cargo.toml` — line 7
- `crates/hl7v2-escape/Cargo.toml` — line 9
- `crates/hl7v2-faker/Cargo.toml` — line 9
- `crates/hl7v2-gen/Cargo.toml` — line 9
- `crates/hl7v2-json/Cargo.toml` — line 9
- `crates/hl7v2-mllp/Cargo.toml` — line 9
- `crates/hl7v2-model/Cargo.toml` — line 9
- `crates/hl7v2-network/Cargo.toml` — line 9
- `crates/hl7v2-normalize/Cargo.toml` — line 9
- `crates/hl7v2-parser/Cargo.toml` — line 9
- `crates/hl7v2-path/Cargo.toml` — line 9
- `crates/hl7v2-prof/Cargo.toml` — line 9
- `crates/hl7v2-query/Cargo.toml` — line 9
- `crates/hl7v2-server/Cargo.toml` — line 9
- `crates/hl7v2-stream/Cargo.toml` — line 9
- `crates/hl7v2-template-values/Cargo.toml` — line 9
- `crates/hl7v2-template/Cargo.toml` — line 9
- `crates/hl7v2-test-utils/Cargo.toml` — line 7
- `crates/hl7v2-validation/Cargo.toml` — line 9
- `crates/hl7v2-writer/Cargo.toml` — line 9
- `xtask/Cargo.toml` — line 8

Full license field scan saved to: `docs/governance/licensing/receipts/cargo-toml-license-fields.txt`

---

## E. Dependency Policy Status

### cargo-deny Configuration (`deny.toml`)

Allowed licenses for **dependencies**:
- MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause
- ISC, Unicode-3.0, Zlib, Unlicense
- CDLA-Permissive-2.0
- AGPL-3.0-or-later (for self-references)

### cargo deny check Results

```
advisories ok, bans ok, licenses ok, sources ok
Exit code: 0
```

Note: `cargo deny` reported a duplicate advisory for `cpufeatures` (0.2.17 and 0.3.0 in lockfile) — this is a non-blocking warning about duplicate entries, not a license issue.

### CI Enforcement

`.github/workflows/security.yml` (lines 106-124) runs a regression check on every push that blocks any MIT/Apache self-claims in the repository.

Full cargo deny output saved to: `docs/governance/licensing/receipts/cargo-deny-output.txt`

---

## F. Recommendation: **PUBLISH NOW**

| Action | Required? |
|---|---|
| Publish to crates.io | **GO** — all surfaces are AGPL-consistent |
| Fix current files | **No** — nothing to fix |
| Update release notes | **No** — v1.2.0 notes already state AGPL-3.0-or-later |
| Fix release artifacts | **No** — no binary artifacts exist |
| History rewrite | **No** — MIT/Apache existed in metadata for 2 unpublished commits while LICENSE was always AGPL; no secrets, no third-party relicensing issues |

---

## G. Verification Checklist

- [x] `cargo deny check` passes (advisories ok, bans ok, licenses ok, sources ok)
- [x] CI regression check pattern matches zero actual license claims
- [x] All 30 Cargo.toml files resolve to `AGPL-3.0-or-later`
- [x] LICENSE file at root is full AGPL-3.0 text (present since initial commit)
- [x] GitHub release v1.2.0 explicitly states AGPL-3.0-or-later
- [x] Tag v1.2.0 includes LICENSE file with AGPL text
- [x] No SPDX headers or license comments in .rs source files claim non-AGPL
- [x] THIRD_PARTY_NOTICES.md correctly attributes dependency licenses

---

## H. Raw Receipt Files

All raw scan outputs are preserved in `docs/governance/licensing/receipts/`:

| File | Contents |
|---|---|
| `current-tree-license-scan.txt` | `rg` output for MIT/Apache in current tree |
| `cargo-toml-license-fields.txt` | License fields from all Cargo.toml files |
| `history-license-search.txt` | Git history search for MIT/Apache references |
| `commit-message-scan.txt` | Commit messages referencing MIT/Apache/AGPL |
| `cargo-deny-output.txt` | Full `cargo deny check` output |
| `tag-annotation-scan.txt` | Tag v1.2.0 annotation and LICENSE content |
| `release-notes.txt` | GitHub release v1.2.0 notes |
| `user-facing-surface-scan.txt` | Exact-phrase scan of user-facing files (addendum) |
| `tag-annotations-full.txt` | Full tag annotation dump (addendum) |

---

## I. User-Facing Surface Verification Addendum

**Date:** 2026-03-07
**Methodology:** Exact licensing phrases scanned across user-facing files only (README, CHANGELOG, CONTRIBUTING, CLA, THIRD_PARTY_NOTICES, docs/, schemas/, .github/, examples/, infrastructure/, DEPLOYMENT, NIX_USAGE). No substring false positives possible.

**Pattern used:**
```
(AGPL-3\.0-or-later|GNU Affero General Public License|MIT OR Apache|MIT License|
 Apache License(, Version 2\.0)?|SPDX-License-Identifier:\s*(MIT|Apache-2\.0|AGPL-3\.0-or-later))
```

### Surface-by-Surface Results

| File | Line(s) | Text (excerpt) | Classification |
|---|---|---|---|
| `README.md` | 328-329, 335 | "GNU Affero General Public License... **AGPL-3.0-or-later**" | AGPL self-licensing |
| `CHANGELOG.md` | 240-241 | "GNU Affero General Public License... (**AGPL-3.0-or-later**)" | AGPL self-licensing |
| `CONTRIBUTING.md` | 11, 14 | "licensed under **AGPL-3.0-or-later**" | AGPL self-licensing |
| `CLA.md` | 23 | "terms of **AGPL-3.0-or-later** (GNU Affero General Public License..." | AGPL self-licensing |
| `examples/README.md` | 262 | "licensed under the same terms (AGPL-3.0-or-later)" | AGPL self-licensing |
| `schemas/openapi/hl7v2-api.yaml` | 38 | `name: AGPL-3.0-or-later` | AGPL self-licensing |
| `THIRD_PARTY_NOTICES.md` | 7 | "**MIT License**: Used by many core Rust ecosystem crates" | Third-party attribution |
| `THIRD_PARTY_NOTICES.md` | 8 | "**Apache License 2.0**: Used by many core Rust ecosystem crates" | Third-party attribution |
| `.github/workflows/security.yml` | 115 | `rg -n "(\bMIT\b\|MIT OR Apache\|Apache-2\.0\|..."` | Dependency policy (CI regression check) |

### Tag & Release Verification

**Tag `v1.2.0` annotation:**
```
Release v1.2.0: Production Readiness & Architectural Finalization
```
No license language in tag annotation — correct (license is in the release body, not the tag message).

**GitHub Release `v1.2.0`:** Previously verified in Section C — body states "License: AGPL-3.0-or-later (see LICENSE)".

### Three Key Answers

1. **Does any current user-facing file describe the project as MIT or Apache?** — **No.** The only MIT/Apache mentions are third-party attribution (`THIRD_PARTY_NOTICES.md:7-8`) and the CI regression check pattern (`.github/workflows/security.yml:115`).

2. **Do release notes or tags contain stale self-licensing language?** — **No.** The v1.2.0 tag annotation contains no license text; the release body correctly states AGPL-3.0-or-later.

3. **Are all current self-licensing statements consistently AGPL-3.0-or-later?** — **Yes.** Four files previously used the shorthand "AGPL-3.0" (without "-or-later") and were corrected in this addendum pass (see below).

### Shorthand Inconsistencies Found and Fixed

Four user-facing files used "AGPL-3.0" instead of the full SPDX identifier "AGPL-3.0-or-later":

| File | Line | Before | After |
|---|---|---|---|
| `infrastructure/grafana/README.md` | 440 | "licensed under AGPL-3.0." | "licensed under AGPL-3.0-or-later." |
| `crates/hl7v2-server/tests/README.md` | 347 | "licensed under AGPL-3.0." | "licensed under AGPL-3.0-or-later." |
| `DEPLOYMENT.md` | 714 | "licensed under AGPL-3.0." | "licensed under AGPL-3.0-or-later." |
| `NIX_USAGE.md` | 336 | "licensed under AGPL-3.0." | "licensed under AGPL-3.0-or-later." |

All four have been corrected. No blockers remain.

### Receipt Files

Raw outputs saved to `docs/governance/licensing/receipts/`:
- `user-facing-surface-scan.txt` — full exact-phrase scan output
- `tag-annotations-full.txt` — complete tag annotation dump

**Note:** The second-pass release-notes JSON generation (`release-notes-full.json`) fell back to an error locally due to `gh` CLI unavailability. This file is excluded from durable receipts. Release-note verification relied on the earlier successful receipt (`release-notes.txt`).
