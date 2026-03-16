# ADR-0006: OPA for Policy Enforcement

**Status**: Proposed

**Date**: 2025-11-19

**Deciders**: Architecture team

**Technical Story**: Healthcare messaging systems require policy enforcement that goes beyond structural HL7v2 validation -- compliance rules, PHI handling, access control, and rate limiting must be externalized so that compliance officers and policy administrators can manage them independently of application code.

## Context

The hl7v2-rs system currently performs structural validation through the `hl7v2-validation` crate (rule-based message validation engine) and profile-based validation through `hl7v2-prof`. However, healthcare integration environments impose a broader category of requirements that do not fit neatly into structural validation:

- **Regulatory compliance**: HIPAA mandates audit logging, encryption in transit, and PHI access controls. These rules change with regulatory updates and must be enforceable without recompiling the application.
- **PHI protection policies**: Determining which fields require redaction in logs (PID.3, PID.5, PID.7, PID.11, PID.13, PID.19, NK1.2, NK1.4, NK1.5) is a policy concern, not a parsing concern.
- **Data quality rules**: Business rules like "reject messages with future birth dates" or "warn when patient age exceeds 120 years" are domain-specific policies that vary by deployment.
- **Temporal consistency**: Rules like "admission date must not precede birth date" span multiple segments and represent business logic rather than structural constraints.
- **Operational policies**: Rate limiting, daily message quotas, TLS enforcement in production, and mandatory audit logging are infrastructure-level policies.
- **Separation of concerns**: Developers maintain parsing and transport code; compliance teams should be able to author and update policy rules without requiring Rust expertise or a redeployment cycle.

Traditional approaches embed these rules directly in application code, making them difficult to audit, slow to update, and impossible for non-developers to manage.

## Decision

We propose using **Open Policy Agent (OPA)** with the **Rego** policy language for externalized policy enforcement in hl7v2-rs. OPA provides a general-purpose policy engine that decouples policy decisions from application logic.

A comprehensive set of Rego policies has been prototyped in `infrastructure/policies/validation.rego` (310 lines, package `hl7v2.validation`), and OPA is available in the Nix development shell (`flake.nix` line 64: `open-policy-agent` in devTools). However, **no Rust crate in the workspace currently integrates OPA evaluation** -- the policies exist as standalone reference implementations that have not been wired into the runtime.

The prototyped policies cover:

1. **Message structure validation** -- MSH segment presence, PID requirement for ADT messages
2. **Message type validation** -- Allow-list of accepted types (ADT^A01, ADT^A04, ADT^A08, ADT^A11, ORU^R01, ORM^O01, RDE^O11, SIU^S12)
3. **PHI redaction rules** -- Identification of sensitive fields for log redaction
4. **Data quality rules** -- Age plausibility checks, future birth date rejection
5. **Temporal consistency** -- Admission date vs. birth date ordering
6. **Required fields by message type** -- ADT^A01 requires PID.3, PID.5, PV1.2, PV1.3
7. **Table/code validation** -- Gender codes (HL7 Table 0001), patient class codes (HL7 Table 0004)
8. **Compliance rules** -- Mandatory audit logging and TLS in production
9. **Rate limiting and quotas** -- Daily message count enforcement

## Consequences

### Positive

- **Separation of concerns**: Policy rules are written in Rego, a purpose-built declarative language. Compliance officers can review, audit, and modify policies without touching Rust code.
- **Hot-reloadable policies**: OPA supports policy bundle updates without application restarts, enabling rapid response to regulatory changes.
- **Audit trail**: OPA decision logs provide a complete record of every policy evaluation, which is valuable for HIPAA compliance audits.
- **Testability**: Rego policies can be tested independently with `opa test`, using the same tooling available in the Nix dev shell.
- **Ecosystem**: OPA is a CNCF graduated project with broad industry adoption for policy enforcement in Kubernetes, API gateways, and microservices.
- **Unified policy language**: The same Rego policies could govern HL7v2 message validation, API access control, and infrastructure configuration.

### Negative

- **Operational complexity**: Running OPA as a sidecar or embedded engine adds another component to deploy, monitor, and upgrade.
- **Latency overhead**: Each policy evaluation requires serializing the HL7v2 message to JSON, sending it to OPA, and deserializing the decision. For high-throughput pipelines (thousands of messages per second), this adds measurable latency.
- **Rego learning curve**: While Rego is simpler than Rust, it is an unfamiliar language for most developers and compliance staff. Its datalog-inspired semantics can be counterintuitive.
- **Integration gap**: No Rust OPA client library is as mature as the Go-native OPA SDK. The integration path (sidecar HTTP, WASM-embedded, or FFI) requires careful evaluation.
- **Debugging complexity**: Policy evaluation failures span two systems (Rust application and OPA engine), making root-cause analysis harder.

### Neutral

- **Complementary to existing validation**: OPA policies would complement, not replace, the `hl7v2-validation` crate. Structural validation remains in Rust; policy validation is externalized to OPA.
- **WASM option**: OPA policies can be compiled to WASM and evaluated in-process using `wasmtime`, which would eliminate the sidecar requirement but adds a WASM runtime dependency.
- **Proto file alignment**: The existing `api/proto/hl7v2.proto` `ValidateResponse` message type (with `ValidationIssue` carrying code, message, severity, and location) maps naturally to OPA deny/warn results.

## Alternatives Considered

### Alternative 1: Embedded Rust Policy Engine

**Pros:**
- No external dependencies; policies compile into the binary
- Zero serialization overhead; policies operate directly on Rust data structures
- Full type safety and compile-time checks on policy rules
- Simpler deployment (single binary)

**Cons:**
- Policy changes require recompilation and redeployment
- Rust is not accessible to compliance teams; policy authoring requires developer involvement
- No standardized policy language; custom DSL or builder pattern would be needed
- Testing policies requires the full Rust toolchain

**Why not chosen:**
The primary motivation for OPA is enabling non-developer policy management. An embedded Rust engine defeats this purpose. The `hl7v2-validation` crate already serves as an embedded rule engine for structural validation; OPA addresses the distinct concern of organizational policy enforcement.

### Alternative 2: XACML (eXtensible Access Control Markup Language)

**Pros:**
- OASIS standard with formal specification
- Mature enterprise adoption, especially in healthcare
- Rich attribute-based access control (ABAC) model
- Well-defined Policy Decision Point (PDP) / Policy Enforcement Point (PEP) architecture

**Cons:**
- XML-based; verbose and difficult to read/write
- Limited Rust ecosystem support; no production-quality XACML engine in Rust
- Primarily designed for access control, not general-purpose policy enforcement
- Heavyweight for message validation use cases

**Why not chosen:**
XACML's XML verbosity and narrow focus on access control make it a poor fit for HL7v2 message validation policies. OPA's Rego language is more expressive and easier to write for data validation rules.

### Alternative 3: Cedar (AWS)

**Pros:**
- Rust-native (written in Rust by AWS)
- Formal verification of policy correctness
- Fast evaluation with static analysis
- Clean, readable policy syntax

**Cons:**
- Primarily designed for authorization (permit/forbid), not general data validation
- Relatively new; smaller community and ecosystem than OPA
- AWS-centric tooling and documentation
- Limited support for complex data validation logic (age calculations, temporal checks)

**Why not chosen:**
Cedar excels at authorization decisions but is not designed for the kind of data validation and compliance checking required for HL7v2 messages. The prototyped Rego policies include age calculations, temporal ordering checks, and table lookups that would be awkward to express in Cedar's authorization-focused model.

### Alternative 4: Custom Rule Engine with YAML/TOML Configuration

**Pros:**
- Simple to implement; YAML/TOML is familiar to operations staff
- No external runtime dependency
- Rules can be hot-reloaded from files
- Minimal learning curve

**Cons:**
- Limited expressiveness; complex rules (temporal checks, cross-field validation) are difficult in declarative YAML
- No formal semantics; behavior of complex rules may be ambiguous
- No ecosystem tooling for testing, debugging, or auditing rules
- Tendency to grow into an ad-hoc, poorly-specified DSL over time

**Why not chosen:**
YAML-based rule engines inevitably hit expressiveness limits. The prototyped Rego policies include helper functions (`calculate_age`, `is_future_date`), cross-segment joins (PID birth date vs. PV1 admission date), and set operations (allowed message types, valid codes) that require a real language, not a configuration format.

## Implementation Notes

### Current State

Rego policies are prototyped and OPA is available in the dev shell, but no Rust integration exists. The implementation path has three options:

### Option A: OPA Sidecar (HTTP API)

Deploy OPA as a sidecar container. The Rust application sends policy queries via HTTP:

```rust
// Hypothetical integration -- NOT yet implemented
use reqwest::Client;
use serde_json::Value;

async fn evaluate_policy(
    opa_client: &Client,
    opa_url: &str,
    message_json: &Value,
) -> Result<PolicyDecision, PolicyError> {
    let input = serde_json::json!({
        "input": {
            "segments": message_json["segments"],
            "config": {
                "environment": "production",
                "log_phi": false,
                "audit_logging": true,
                "tls_enabled": true,
                "max_daily_messages": 100000
            }
        }
    });

    let response = opa_client
        .post(format!("{}/v1/data/hl7v2/validation/result", opa_url))
        .json(&input)
        .send()
        .await?;

    let decision: PolicyDecision = response.json().await?;
    Ok(decision)
}
```

### Option B: OPA WASM (In-Process)

Compile Rego policies to WASM and evaluate in-process using `wasmtime`:

```bash
# Compile policies to WASM bundle
opa build -t wasm -e 'hl7v2/validation/result' \
    infrastructure/policies/validation.rego \
    -o policy-bundle.tar.gz
```

```rust
// Hypothetical integration -- NOT yet implemented
use wasmtime::{Engine, Module, Store};

struct PolicyEngine {
    engine: Engine,
    module: Module,
}

impl PolicyEngine {
    fn evaluate(&self, input: &[u8]) -> Result<PolicyDecision, PolicyError> {
        // Load WASM module, set input, evaluate, read output
        todo!("WASM integration not implemented")
    }
}
```

### Option C: opa-wasm Crate (Burrego)

Use the `burrego` crate (Rust OPA evaluator) for in-process evaluation without a sidecar:

```rust
// Hypothetical integration -- NOT yet implemented
use burrego::opa::Opa;

fn evaluate_policy(rego_source: &str, input: &str) -> Result<PolicyDecision, PolicyError> {
    let mut opa = Opa::new()?;
    opa.add_policy("validation", rego_source)?;
    let result = opa.evaluate("data.hl7v2.validation.result", input)?;
    Ok(serde_json::from_value(result)?)
}
```

### Rego Policy Structure

The existing `infrastructure/policies/validation.rego` follows OPA best practices:

- **`deny` rules** produce error-severity violations (message rejected)
- **`warn` rules** produce warning-severity advisories (message accepted with warnings)
- **`allow`** is true when `count(deny) == 0`
- **`result`** aggregates `allow`, `errors`, `warnings`, and counts

```rego
# Example: deny future birth dates
deny[msg] {
    some seg in input.segments
    seg.id == "PID"
    birth_date := seg.fields[7].value
    is_future_date(birth_date)
    msg := {
        "code": "POL_FutureBirthDate",
        "message": sprintf("Birth date %v is in the future", [birth_date]),
        "severity": "error",
        "field": "PID.7"
    }
}
```

### Integration Points

When implemented, OPA evaluation would be invoked:

1. **In `hl7v2-server`**: As middleware or a validation layer after parsing, before returning the HTTP/gRPC response.
2. **In `hl7v2-cli`**: As an optional `--policy` flag on the `validate` subcommand.
3. **In `hl7v2-validation`**: As an optional external policy backend alongside the existing rule engine.

### Next Steps (Not Yet Started)

- [ ] Evaluate integration approach (sidecar vs. WASM vs. burrego)
- [ ] Add OPA client crate to workspace
- [ ] Define `PolicyDecision` types in a shared crate
- [ ] Wire policy evaluation into `hl7v2-server` middleware
- [ ] Add CLI `--policy` flag to `hl7v2-cli validate`
- [ ] Write integration tests with sample HL7v2 messages
- [ ] Benchmark policy evaluation latency

## References

- [Open Policy Agent](https://www.openpolicyagent.org/) -- CNCF graduated project
- [Rego Policy Language](https://www.openpolicyagent.org/docs/latest/policy-language/)
- [OPA WASM](https://www.openpolicyagent.org/docs/latest/wasm/) -- Compiling Rego to WebAssembly
- [Burrego](https://github.com/kubewarden/burrego) -- Rust-native OPA evaluator
- [HIPAA Security Rule](https://www.hhs.gov/hipaa/for-professionals/security/) -- Regulatory context
- `infrastructure/policies/validation.rego` -- Prototyped Rego policies (310 lines)
- `flake.nix` line 64 -- OPA available in Nix dev shell
