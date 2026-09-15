# Hosted CLA Assistant governance

This repository uses **CLA Assistant from `cla-assistant.io`**, the hosted GitHub App. It does not use CLA Assistant Lite or repository-hosted CLA workflow code.

## Activation state

This migration is staged until the hosted service is linked and proven on its pull request. Before merge, maintainers must create the shared public Gist from the reviewed agreement and metadata payloads, link both HL7 repositories separately, observe a successful `license/cla` status in each repository, replace the pending Gist fields in each [`cla-assistant-source.json`](cla-assistant-source.json), and require the status on each default branch with CLA Assistant pinned as its expected source.

## Agreement source

`EffortlessMetrics/hl7v2-rs` and `EffortlessMetrics/hl7v2-rs-swarm` are linked separately to the same public Gist. The Gist contains `CLA.md` and `metadata`; its `CLA.md` must be byte-identical to the repository [`CLA.md`](../../CLA.md) in both repositories. The staged custom-field payload is retained in [`cla-assistant-metadata.json`](cla-assistant-metadata.json). The activation state, exact Gist URL and revision, and content hashes are recorded in [`cla-assistant-source.json`](cla-assistant-source.json).

The required custom fields are full legal name, email address, and an acknowledgement that the signer is acting in an individual capacity and has authority to grant the stated rights.

## Corporate contributions

The hosted form is the individual flow. A contributor whose employer or another entity owns or controls the relevant rights must not sign on the entity's behalf through that form. Corporate contributions require a separate written agreement and authorization process handled privately by the maintainers.

## Enforcement

The two migration pull requests are the initial test pull requests. Each must receive a successful `license/cla` status from the hosted App. Each default-branch ruleset must then require that context and pin its expected source to CLA Assistant. The allowlist starts empty. A bot may be exempted only after its contribution path is shown to be controlled and attributable to the project.

## Evidence retention

Maintainers export the signature register privately when the CLA changes and at release or governance-freeze checkpoints. The retained evidence set includes the exported register, Gist payload and revision, repository source records, ruleset JSON, and checksums. Personal signature data is not committed to either repository.

Changing the agreement or `metadata` creates a new version. Update both repositories and the shared Gist in one governed change, update both source records, expect contributors to re-sign, and take a fresh private export.

See [`cla-privacy.md`](cla-privacy.md) for the contributor privacy notice.
