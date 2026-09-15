# Hosted CLA Assistant Operating Record

**Repository:** `EffortlessMetrics/hl7v2-rs`  
**Status:** Pending activation; this change must remain draft until the activation receipts below are complete.  
**Expected status context:** `license/cla`  
**Repository CLA SHA-256:** `de968d5421513b83e4655995e8e2bb95600a351b2768b89bc70aebedec744191`

## Decision

Use the hosted **CLA Assistant** GitHub App from `cla-assistant.io`. Do not add a repository CLA workflow.

The app enforces [CLA.md](../../CLA.md); it does not supply or expand the agreement. The current `hl7v2-rs` agreement grants contribution rights for distribution under `AGPL-3.0-or-later`. It does **not** add an alternative-commercial-license grant.

## Agreement source

CLA Assistant must present a public Gist whose contents are byte-equivalent to [CLA.md](../../CLA.md).

- **Gist URL:** `PENDING — record before merge`
- **Gist revision:** `PENDING — record before merge`
- **Gist content SHA-256:** `PENDING — must equal the repository CLA SHA-256 above`

Changing the Gist creates a new agreement version and requires contributors to sign that version. Update all three fields whenever the CLA changes.

## Individual signing fields

Require only:

1. full legal name;
2. email address; and
3. acknowledgement: **I am signing in my individual capacity and have authority to grant the rights stated in this Agreement.**

GitHub identity and signature time come from the authenticated signing event.

Contributions owned by an employer or another entity stay outside this individual flow. The maintainer must establish a separate written contribution agreement or authorization before accepting them.

## Enforcement

1. Install the hosted CLA Assistant GitHub App on this repository only.
2. Start with no human, collaborator, or organization-member exemptions.
3. Add an automation account to the allowlist only when it actually authors project-controlled contributions and its provenance is documented.
4. Open a test pull request and verify both unsigned and signed behavior.
5. Confirm that the app emits `license/cla`.
6. Add `license/cla` to the `main` ruleset as a required status check.
7. Pin the required check to the CLA Assistant App as its expected source.
8. Record the test pull request and ruleset receipt below.

- **Activation test PR:** `PENDING`
- **Ruleset receipt:** `PENDING`
- **Expected-source receipt:** `PENDING`

## Evidence retention

Export the hosted signature register privately:

- whenever the CLA or Gist revision changes;
- at release or governance freeze checkpoints; and
- before replacing or discontinuing the service.

Store exports outside the public repository with a checksum and the corresponding Gist revision. The hosted database must not be the sole licensing record.

Contributor data handling is described in [CONTRIBUTOR_PRIVACY.md](../../CONTRIBUTOR_PRIVACY.md).
