# Full-History AGPL Release Readiness Decision Memo

**Repository:** EffortlessMetrics/hl7v2-rs
**Date:** 2026-03-07
**Scope:** Full git history release under AGPL-3.0-or-later
**Decision:** **PRESERVE FULL HISTORY AS-IS -- no rewrite needed**

---

## 1. Audit Scope

This memo extends the existing publish-readiness audit (`docs/governance/licensing-audit-2026-03-06.md`) with a deep, blocker-focused review of five additional surfaces:

| # | Surface | Question Answered |
|---|---------|-------------------|
| 1 | **Deep git history** | Does any commit contain a real licensing conflict (license files, boilerplate, SPDX headers)? |
| 2 | **Public releases** | Do tagged releases or GitHub Release artifacts carry inconsistent licensing? |
| 3 | **crates.io publications** | Are there immutable published artifacts with wrong license metadata? |
| 4 | **Mirrors and deployment** | Do any mirrors, Docker images, or deployment configs propagate stale claims? |
| 5 | **Third-party notice integrity** | Is `THIRD_PARTY_NOTICES.md` accurate and complete? |

The default assumption is: **preserve history unless a real blocker requires rewrite**.

---

## 2. Classification Report

| Surface | Classification | Detail |
|---------|---------------|--------|
| `LICENSE` (root) | CLEAN | Full AGPL-3.0 text since initial commit `1aa9ab5` |
| Root `Cargo.toml` license field | CLEAN | `"AGPL-3.0-or-later"` (line 45) |
| All 30 crate `Cargo.toml` files | CLEAN | All use `license.workspace = true` |
| README / CHANGELOG / CONTRIBUTING / CLA | CLEAN | All state AGPL-3.0-or-later |
| OpenAPI spec | CLEAN | `name: AGPL-3.0-or-later` |
| `deny.toml` allow-list | INTENTIONAL | Dependency policy, not self-licensing |
| `THIRD_PARTY_NOTICES.md` | INTENTIONAL | Correctly attributes dependency licenses |
| CI regression check (`security.yml`) | INTENTIONAL | Active gate blocks MIT/Apache self-claims |
| Commit `d04e739` (2nd) Cargo.toml | HISTORICAL-ONLY | `license = "MIT OR Apache-2.0"` -- scaffold placeholder, never published/tagged |
| Commit `8ad8fa6` (3rd) Cargo.toml | HISTORICAL-ONLY | Fixed to `license = "AGPL v3"` -- the corrective commit |
| Commit messages mentioning MIT/Apache | INTENTIONAL | Describe the license change, not false claims |
| Tag `v1.2.0` | CLEAN | AGPL-3.0-or-later in Cargo.toml + LICENSE at tag |
| GitHub Release v1.2.0 | CLEAN | Body states "License: AGPL-3.0-or-later" |
| crates.io publications | CLEAN | **No crates published** -- no immutable artifacts |
| Historical license files (LICENSE-MIT, etc.) | CLEAN | **Never existed** in any commit |
| MIT boilerplate text | CLEAN | **Never existed** in any file |
| Mirrors / deployment configs | CLEAN | Single canonical GitHub remote, no mirrors |
| Docker / container / deployment configs | CLEAN | Dockerfile, docker-compose, K8s manifests, Grafana, OPA policies, and deployment docs exist in-repo; all correctly reference AGPL-3.0-or-later. No published container images, registry artifacts, or distributed bundles were found. |
| Existing audit report + receipts | CLEAN | Comprehensive, 9 receipt files |

---

## 3. Deep History Scan Results

### 3.1 Rev-list search for license files

```
git log --all --oneline -- LICENSE-MIT LICENSE-APACHE LICENSE-BSD
```

**Result:** No output. These files never existed in any commit across any branch.

### 3.2 LICENSE file provenance

```
git log --diff-filter=A -- LICENSE
```

The `LICENSE` file was added in the initial commit (`1aa9ab5`) and has always contained the full GNU Affero General Public License v3 text. It has never been modified.

### 3.3 Cargo.toml license field history

The workspace `Cargo.toml` license field went through three states:

| Commit | Value | Duration | Published? |
|--------|-------|----------|------------|
| `d04e739` (2nd commit) | `"MIT OR Apache-2.0"` | 1 commit | No |
| `8ad8fa6` (3rd commit) | `"AGPL v3"` | Several commits | No |
| `82156fc` | `"AGPL-3.0-or-later"` | Current | No (crates.io) |

MIT/Apache-2.0 appeared only in brief early unpublished Cargo.toml metadata (commits `d04e739` to `8ad8fa6`) before correction; no tag, release, or published artifact carried it. During this period:
- The LICENSE file was already AGPL-3.0 (the actual license grant)
- No crates were published
- No tags were created
- No releases were made

### 3.4 MIT boilerplate scan

No file in any commit ever contained MIT boilerplate text ("Permission is hereby granted, free of charge..."). The existing audit receipt (`docs/governance/licensing/receipts/history-license-search.txt`) confirms this via `git log -S` searches.

### 3.5 SPDX header scan

No `.rs` source files contain SPDX license identifiers or license headers claiming non-AGPL licensing, in any commit.

---

## 4. Tag and Release Checks

### 4.1 Tags

```
git tag -l
```

**Result:** Single tag `v1.2.0`.

**Tag `v1.2.0` verification:**
- Tag annotation: "Release v1.2.0: Production Readiness & Architectural Finalization"
- `git show v1.2.0:LICENSE` = full AGPL-3.0 text
- `git show v1.2.0:Cargo.toml` contains `license = "AGPL-3.0-or-later"`
- No license language in tag annotation body (correct -- license is in the release body)

### 4.2 GitHub Release

GitHub Release `v1.2.0` body explicitly states: "License: AGPL-3.0-or-later (see LICENSE)".
No binary artifacts attached. Source-only release.

Receipt: `docs/governance/licensing/receipts/release-notes.txt`

---

## 5. crates.io Publication Check

**No crates from this workspace have been published to crates.io.**

This means:
- No immutable registry artifacts exist with any license metadata
- No `cargo install` or `cargo add` consumers have received MIT/Apache-claimed packages
- The first publication will carry correct AGPL-3.0-or-later metadata

---

## 6. Mirror and Deployment Check

```
git remote -v
```

**Result:** Single remote `origin` pointing to `git@github.com:EffortlessMetrics/hl7v2-rs.git`.

- No secondary remotes or mirrors configured
- Docker, Kubernetes, and deployment configuration files exist in `infrastructure/` and `DEPLOYMENT.md`; all correctly reference AGPL-3.0-or-later
- CI/CD workflows exist in `.github/workflows/`; none publish container images or binary artifacts
- No published container images, registry artifacts, or deployment bundles were found carrying any license metadata

---

## 7. Third-Party Notice Integrity

`THIRD_PARTY_NOTICES.md` correctly attributes dependency licenses (MIT, Apache-2.0, BSD, ISC, etc.) for third-party crates. The `cargo deny check` command passes cleanly:

```
advisories ok, bans ok, licenses ok, sources ok
```

The `deny.toml` allow-list includes MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Zlib, Unlicense, CDLA-Permissive-2.0, and AGPL-3.0-or-later (for self). All are permissive licenses compatible with AGPL-3.0-or-later distribution.

Receipt: `docs/governance/licensing/receipts/cargo-deny-output.txt`

---

## 8. Decision and Recommendation

### Decision: PRESERVE FULL HISTORY AS-IS

No history rewrite is needed. The repository's full development history can be released under AGPL-3.0-or-later without modification.

### Rationale

1. **The LICENSE file was AGPL from the very first commit.** The actual license grant -- the only legally operative document -- has never been anything other than AGPL-3.0.

2. **MIT/Apache-2.0 appeared only in a brief early unpublished Cargo.toml metadata state** (commits `d04e739` to `8ad8fa6`). For this repository, the decisive current licensing evidence is the root `LICENSE` plus the current package metadata; the brief early metadata drift never reached a published artifact.

3. **No LICENSE-MIT or LICENSE-APACHE files ever existed** in any commit on any branch. No MIT boilerplate text was ever present in any file.

4. **No crates were published to crates.io.** There are no immutable publication artifacts that could carry conflicting metadata.

5. **No mirrors or published deployment artifacts need reseeding.** Single canonical GitHub remote. Deployment configuration exists in-repo (Docker, K8s, Nix) and correctly references AGPL-3.0-or-later, but no published container images, registry artifacts, or mirrored distribution surfaces were found.

6. **CI regression check actively prevents future drift.** The `.github/workflows/security.yml` workflow blocks any MIT/Apache self-claims on every push.

7. **The existing audit report documents the 2-commit boundary** with full receipts, providing a complete paper trail.

### What would require a rewrite (none found)

A history rewrite would be required if any of the following were true:

| Condition | Found? |
|-----------|--------|
| A LICENSE-MIT or LICENSE-APACHE file existed in any historical commit | No |
| MIT/Apache boilerplate text appeared in source files | No |
| SPDX headers in source files claimed non-AGPL licensing | No |
| Crates were published to crates.io with MIT/Apache metadata | No |
| Binary artifacts were distributed under MIT/Apache terms | No |
| Third-party contributions were accepted under MIT/Apache CLA terms | No |
| Mirror repositories propagated MIT/Apache claims | No |

None of these conditions were met.

---

## 9. Supporting Evidence

### Existing audit report

`docs/governance/licensing-audit-2026-03-06.md` -- comprehensive publish-readiness audit covering current-tree status, history classification, release artifacts, Cargo.toml inventory, dependency policy, and user-facing surface verification.

### Receipt files

All raw scan outputs in `docs/governance/licensing/receipts/`:

| File | Contents |
|------|----------|
| `current-tree-license-scan.txt` | `rg` output for MIT/Apache in current tree |
| `cargo-toml-license-fields.txt` | License fields from all 30 Cargo.toml files |
| `history-license-search.txt` | Git history search for MIT/Apache references |
| `commit-message-scan.txt` | Commit messages referencing MIT/Apache/AGPL |
| `cargo-deny-output.txt` | Full `cargo deny check` output |
| `tag-annotation-scan.txt` | Tag v1.2.0 annotation and LICENSE content |
| `tag-annotations-full.txt` | Full tag annotation dump |
| `release-notes.txt` | GitHub Release v1.2.0 notes |
| `user-facing-surface-scan.txt` | Exact-phrase scan of user-facing files |
