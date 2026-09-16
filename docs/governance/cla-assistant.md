# Hosted CLA Assistant governance

This repository uses **CLA Assistant from `cla-assistant.io`**, the hosted GitHub App. It does not use CLA Assistant Lite or repository-hosted CLA workflow code.

## Activation state

This migration is staged until the hosted service is linked and proven on its pull request. Before merge, maintainers must create the shared public Gist from the reviewed agreement and metadata payloads, link both HL7 repositories separately, observe a successful `license/cla` status in each repository, replace the pending Gist fields in each [`cla-assistant-source.json`](cla-assistant-source.json), and require the status on each default branch with CLA Assistant pinned as its expected source.

## Agreement source

`EffortlessMetrics/hl7v2-rs` and `EffortlessMetrics/hl7v2-rs-swarm` are linked separately to the same public Gist. The Gist contains `CLA.md` and `metadata`; its `CLA.md` must be byte-identical to the repository [`CLA.md`](../../CLA.md) in both repositories. The staged custom-field payload is retained in [`cla-assistant-metadata.json`](cla-assistant-metadata.json). The activation state, exact Gist URL and revision, and content hashes are recorded in [`cla-assistant-source.json`](cla-assistant-source.json).

The required custom fields are full legal name, email address, and an acknowledgement that the signer is acting in an individual capacity and has authority to grant the stated rights.

## Corporate contributions

The hosted form is the individual flow. A contributor whose employer or another entity owns or controls the relevant rights must not sign on the entity's behalf through that form. Corporate contributions require a separate written agreement and authorization process handled privately by the maintainers.

A corporate contribution does not create a false individual signature and does not put a human account or employer organization on the CLA Assistant allowlist. After a Corporate CLA is executed, the private authorization record must identify the entity, agreement version, covered GitHub usernames, scope, and effective date.

The corporate merge path is an audited exception to the CLA rule only:

1. enforce the App-pinned `license/cla` check in a dedicated default-branch CLA ruleset, separate from the repository's baseline branch ruleset;
2. make a dedicated `cla-corporate-approvers` team the only `pull_request`-mode bypass actor on that CLA-only ruleset;
3. require an approver to verify the private Corporate CLA record before bypassing the CLA ruleset for a specific pull request; and
4. retain a private receipt containing the entity, agreement version, covered usernames, pull request, approver, timestamp, and reason.

All ordinary pull-request, review, CI, deletion, and non-fast-forward rules remain enforced. Until the Corporate CLA and the narrowly scoped exception path are both configured, an entity-owned contribution cannot merge.

## Enforcement

The two migration pull requests are the initial test pull requests. Each must receive a successful `license/cla` status from the hosted App. Each repository must then enforce that context through a dedicated default-branch CLA ruleset and pin its expected source to CLA Assistant. Keep each existing baseline ruleset unchanged and without new bypass actors. Each CLA-only ruleset starts with no bypass actor; add only the dedicated corporate-approval team if the corporate process described above is activated.

The CLA Assistant allowlist starts empty. A bot may be exempted only after its contribution path is shown to be controlled and attributable to the project. Do not use that allowlist for collaborators, organization members, or corporate contributors.

## Evidence retention

Maintainers export the signature register privately when the CLA changes and at release or governance-freeze checkpoints. The retained evidence set includes the exported register, Gist payload and revision, repository source records, ruleset JSON, and checksums. Personal signature data is not committed to either repository.

Changing the agreement or `metadata` creates a new version. Update both repositories and the shared Gist in one governed change, update both source records, expect contributors to re-sign, and take a fresh private export.

See [`cla-privacy.md`](cla-privacy.md) for the contributor privacy notice.
