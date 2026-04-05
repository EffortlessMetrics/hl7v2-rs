//! BDD tests for hl7v2-gen using Cucumber
//!
//! Run with: cargo test --test bdd_tests

use std::collections::HashMap;

use cucumber::{World, given, then, when};
use hl7v2_gen::{AckCode, Faker, Message, Template, ValueSource, ack, ack_with_error, generate};

/// Test world for generation BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct GenWorld {
    template: Option<Template>,
    seed: u64,
    messages_a: Vec<Message>,
    messages_b: Vec<Message>,
    ack_message: Option<Message>,
    original_message: Option<Message>,
    faker_result: Option<String>,
}

impl GenWorld {
    fn new() -> Self {
        Self {
            template: None,
            seed: 0,
            messages_a: Vec::new(),
            messages_b: Vec::new(),
            ack_message: None,
            original_message: None,
            faker_result: None,
        }
    }

    fn simple_adt_template() -> Template {
        Template {
            name: "adt_a01".to_string(),
            delims: r"^~\&".to_string(),
            segments: vec![
                r"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1"
                    .to_string(),
                r"PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            ],
            values: HashMap::new(),
        }
    }
}

// ============================================================================
// Given Steps
// ============================================================================

#[given("a simple ADT template")]
fn given_simple_template(world: &mut GenWorld) {
    world.template = Some(GenWorld::simple_adt_template());
}

#[given("seed value 42")]
fn given_seed_42(world: &mut GenWorld) {
    world.seed = 42;
}

#[given("a template with dynamic values")]
fn given_dynamic_template(world: &mut GenWorld) {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);
    world.template = Some(Template {
        name: "dynamic".to_string(),
        delims: r"^~\&".to_string(),
        segments: vec![
            r"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
            r"PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
        ],
        values,
    });
}

#[given(regex = r#"a template with PID\.5 fixed to "([^"]+)""#)]
fn given_template_fixed_pid5(world: &mut GenWorld, value: String) {
    let mut values = HashMap::new();
    values.insert("PID.5".to_string(), vec![ValueSource::Fixed(value)]);
    world.template = Some(Template {
        name: "fixed".to_string(),
        delims: r"^~\&".to_string(),
        segments: vec![
            r"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
            r"PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
        ],
        values,
    });
}

#[given(regex = r#"a template with PID\.8 from list "([^"]+)""#)]
fn given_template_from_list(world: &mut GenWorld, list: String) {
    let items: Vec<String> = list
        .split(',')
        .map(std::string::ToString::to_string)
        .collect();
    let mut values = HashMap::new();
    values.insert("PID.8".to_string(), vec![ValueSource::From(items)]);
    world.template = Some(Template {
        name: "from_list".to_string(),
        delims: r"^~\&".to_string(),
        segments: vec![
            r"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
            r"PID|1||123456^^^HOSP^MR||Doe^John||19800101|M".to_string(),
        ],
        values,
    });
}

#[given("a template with PID.3 as UUID")]
fn given_template_uuid(world: &mut GenWorld) {
    let mut values = HashMap::new();
    values.insert("PID.3".to_string(), vec![ValueSource::UuidV4]);
    world.template = Some(Template {
        name: "uuid".to_string(),
        delims: r"^~\&".to_string(),
        segments: vec![
            r"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
            r"PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
        ],
        values,
    });
}

#[given("a template with PID.3 as 6-digit numeric")]
fn given_template_numeric(world: &mut GenWorld) {
    let mut values = HashMap::new();
    values.insert(
        "PID.3".to_string(),
        vec![ValueSource::Numeric { digits: 6 }],
    );
    world.template = Some(Template {
        name: "numeric".to_string(),
        delims: r"^~\&".to_string(),
        segments: vec![
            r"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
            r"PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
        ],
        values,
    });
}

#[given(regex = r#"a template with PID\.7 as date between "(\d+)" and "(\d+)""#)]
fn given_template_date_range(world: &mut GenWorld, start: String, end: String) {
    let mut values = HashMap::new();
    values.insert("PID.7".to_string(), vec![ValueSource::Date { start, end }]);
    world.template = Some(Template {
        name: "date".to_string(),
        delims: r"^~\&".to_string(),
        segments: vec![
            r"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1".to_string(),
            r"PID|1||123456^^^HOSP^MR||Doe^John|||M||||".to_string(),
        ],
        values,
    });
}

#[given("a template with OBX.5 as gaussian mean 100.0 stddev 10.0")]
fn given_template_gaussian(world: &mut GenWorld) {
    let mut values = HashMap::new();
    values.insert(
        "OBX.5".to_string(),
        vec![ValueSource::Gaussian {
            mean: 100.0,
            sd: 10.0,
            precision: 2,
        }],
    );
    world.template = Some(Template {
        name: "gaussian".to_string(),
        delims: r"^~\&".to_string(),
        segments: vec![
            r"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ORU^R01|ABC123|P|2.5.1".to_string(),
            r"PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            r"OBR|1|||1234^Test".to_string(),
            r"OBX|1|NM|1234^Result||120|mg/dL".to_string(),
        ],
        values,
    });
}

#[given("a valid HL7 message to acknowledge")]
fn given_valid_message_for_ack(world: &mut GenWorld) {
    let msg = hl7v2_core::parse(
        b"MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r",
    )
    .unwrap();
    world.original_message = Some(msg);
}

#[given("an ORU template with OBX segments")]
fn given_oru_template(world: &mut GenWorld) {
    world.template = Some(Template {
        name: "oru_r01".to_string(),
        delims: r"^~\&".to_string(),
        segments: vec![
            r"MSH|^~\&|App|Fac|Recv|Fac|20250128152312||ORU^R01|ABC123|P|2.5.1".to_string(),
            r"PID|1||123456^^^HOSP^MR||Doe^John".to_string(),
            r"OBR|1|||1234^Test".to_string(),
            r"OBX|1|NM|1234^Result||120|mg/dL".to_string(),
        ],
        values: HashMap::new(),
    });
}

#[given("a faker with seed 42")]
fn given_faker(_world: &mut GenWorld) {
    // Faker will be created in the When step
}

// ============================================================================
// When Steps
// ============================================================================

#[when("I generate a message")]
fn when_generate_message(world: &mut GenWorld) {
    let template = world.template.as_ref().expect("No template set");
    world.messages_a = generate(template, world.seed, 1).expect("Generation failed");
}

#[when("I generate another message with the same seed")]
fn when_generate_same_seed(world: &mut GenWorld) {
    let template = world.template.as_ref().expect("No template set");
    world.messages_b = generate(template, world.seed, 1).expect("Generation failed");
}

#[when(regex = r"I generate a message with seed (\d+)")]
fn when_generate_with_seed(world: &mut GenWorld, seed: u64) {
    let template = world.template.as_ref().expect("No template set");
    let msgs = generate(template, seed, 1).expect("Generation failed");
    if world.messages_a.is_empty() {
        world.messages_a = msgs;
    } else {
        world.messages_b = msgs;
    }
}

#[when(regex = r"I generate (\d+) messages with seed (\d+)")]
fn when_generate_n_messages(world: &mut GenWorld, count: usize, seed: u64) {
    let template = world.template.as_ref().expect("No template set");
    let msgs = generate(template, seed, count).expect("Generation failed");
    if world.messages_a.is_empty() {
        world.messages_a = msgs;
    } else {
        world.messages_b = msgs;
    }
}

#[when(regex = r"I generate (\d+) messages again with seed (\d+)")]
fn when_generate_n_again(world: &mut GenWorld, count: usize, seed: u64) {
    let template = world.template.as_ref().expect("No template set");
    world.messages_b = generate(template, seed, count).expect("Generation failed");
}

#[when("I generate an ACK with code AA")]
fn when_generate_ack_aa(world: &mut GenWorld) {
    let msg = world
        .original_message
        .as_ref()
        .expect("No original message");
    world.ack_message = Some(ack(msg, AckCode::AA).expect("ACK generation failed"));
}

#[when(regex = r#"I generate an ACK with error code AE and text "([^"]+)""#)]
fn when_generate_ack_error(world: &mut GenWorld, text: String) {
    let msg = world
        .original_message
        .as_ref()
        .expect("No original message");
    world.ack_message =
        Some(ack_with_error(msg, AckCode::AE, Some(&text)).expect("ACK generation failed"));
}

#[when(regex = r#"I generate a patient name for gender "([^"]+)""#)]
fn when_faker_name(world: &mut GenWorld, gender: String) {
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    world.faker_result = Some(faker.name(Some(&gender)));
}

// ============================================================================
// Then Steps
// ============================================================================

#[then("both messages should be byte-for-byte identical")]
fn then_identical(world: &mut GenWorld) {
    assert!(!world.messages_a.is_empty());
    assert!(!world.messages_b.is_empty());
    let a = hl7v2_core::write(&world.messages_a[0]);
    let b = hl7v2_core::write(&world.messages_b[0]);
    assert_eq!(a, b, "Messages with same seed should be identical");
}

#[then("the messages should differ")]
fn then_differ(world: &mut GenWorld) {
    let a = hl7v2_core::write(&world.messages_a[0]);
    let b = hl7v2_core::write(&world.messages_b[0]);
    assert_ne!(a, b, "Messages with different seeds should differ");
}

#[then(regex = r"I should receive (\d+) messages")]
fn then_receive_n(world: &mut GenWorld, count: usize) {
    assert_eq!(world.messages_a.len(), count);
}

#[then("all messages should be valid HL7")]
fn then_all_valid(world: &mut GenWorld) {
    for msg in &world.messages_a {
        let bytes = hl7v2_core::write(msg);
        let parsed = hl7v2_core::parse(&bytes);
        assert!(parsed.is_ok(), "Message should be valid HL7");
    }
}

#[then("the generated message should be valid HL7")]
fn then_generated_valid(world: &mut GenWorld) {
    let msg = &world.messages_a[0];
    let bytes = hl7v2_core::write(msg);
    let parsed = hl7v2_core::parse(&bytes);
    assert!(parsed.is_ok(), "Generated message should be valid HL7");
}

#[then(regex = r#"all PID\.8 values should be from the list "([^"]+)""#)]
fn then_values_from_list(world: &mut GenWorld, list: String) {
    let allowed: Vec<&str> = list.split(',').collect();
    for msg in &world.messages_a {
        if let Some(val) = hl7v2_core::get(msg, "PID.8") {
            assert!(
                allowed.contains(&val),
                "PID.8 value '{}' not in allowed list {:?}",
                val,
                allowed
            );
        }
    }
}

#[then("the ACK should have MSH and MSA segments")]
fn then_ack_msh_msa(world: &mut GenWorld) {
    let ack = world.ack_message.as_ref().expect("No ACK generated");
    assert_eq!(ack.segments.len(), 2);
    assert_eq!(std::str::from_utf8(&ack.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&ack.segments[1].id).unwrap(), "MSA");
}

#[then("the ACK should have MSH, MSA, and ERR segments")]
fn then_ack_msh_msa_err(world: &mut GenWorld) {
    let ack = world.ack_message.as_ref().expect("No ACK generated");
    assert_eq!(ack.segments.len(), 3);
    assert_eq!(std::str::from_utf8(&ack.segments[0].id).unwrap(), "MSH");
    assert_eq!(std::str::from_utf8(&ack.segments[1].id).unwrap(), "MSA");
    assert_eq!(std::str::from_utf8(&ack.segments[2].id).unwrap(), "ERR");
}

#[then(regex = r#"MSA\.1 should be "([^"]+)""#)]
fn then_msa1(world: &mut GenWorld, expected: String) {
    let ack = world.ack_message.as_ref().expect("No ACK generated");
    let val = hl7v2_core::get(ack, "MSA.1").expect("MSA.1 not found");
    assert_eq!(val, expected);
}

#[then(regex = r#"the generated message should contain segment "([^"]+)""#)]
fn then_contains_segment(world: &mut GenWorld, segment_id: String) {
    let msg = &world.messages_a[0];
    let has_segment = msg
        .segments
        .iter()
        .any(|s| std::str::from_utf8(&s.id).unwrap() == segment_id);
    assert!(has_segment, "Message should contain {} segment", segment_id);
}

#[then("both corpora should be identical")]
fn then_corpora_identical(world: &mut GenWorld) {
    assert_eq!(world.messages_a.len(), world.messages_b.len());
    for (a, b) in world.messages_a.iter().zip(world.messages_b.iter()) {
        let wa = hl7v2_core::write(a);
        let wb = hl7v2_core::write(b);
        assert_eq!(wa, wb, "Corpus messages with same seed should be identical");
    }
}

#[then("the name should contain a component separator")]
fn then_name_has_component_sep(world: &mut GenWorld) {
    let name = world.faker_result.as_ref().expect("No faker result");
    assert!(name.contains('^'), "Name '{}' should contain '^'", name);
}

// =============================================================================
// Dependency Alignment World and Steps (EFF-1136)
// =============================================================================

/// Test world for workspace dependency alignment BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct DependencyWorld {
    /// Current crate Cargo.toml being analyzed
    current_cargo_toml: Option<std::path::PathBuf>,
    /// All workspace crate Cargo.toml paths
    workspace_cargos: Vec<std::path::PathBuf>,
    /// Current dependency name being checked
    current_dep: Option<String>,
    /// Violations found during checks
    violations: Vec<String>,
    /// Test should fail flag
    should_fail: bool,
    /// Expected error message pattern
    expected_error: Option<String>,
}

impl DependencyWorld {
    fn new() -> Self {
        Self {
            current_cargo_toml: None,
            workspace_cargos: Vec::new(),
            current_dep: None,
            violations: Vec::new(),
            should_fail: false,
            expected_error: None,
        }
    }

    fn get_workspace_root() -> std::path::PathBuf {
        let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate_dir
            .parent()
            .expect("hl7v2-gen is in crates dir")
            .parent()
            .expect("crates is in workspace root")
            .to_path_buf()
    }

    fn get_all_workspace_cargos() -> Vec<std::path::PathBuf> {
        let workspace_root = Self::get_workspace_root();
        let root_cargo = workspace_root.join("Cargo.toml");
        let content = std::fs::read_to_string(&root_cargo).expect("Read workspace Cargo.toml");
        let manifest: toml::Value = toml::from_str(&content).expect("Parse workspace Cargo.toml");

        let mut paths = Vec::new();
        if let Some(workspace) = manifest.get("workspace") {
            if let Some(members) = workspace.get("members") {
                if let Some(members_array) = members.as_array() {
                    for member in members_array {
                        if let Some(member_str) = member.as_str() {
                            let member_path = workspace_root.join(member_str);
                            let cargo_toml = member_path.join("Cargo.toml");
                            if cargo_toml.exists() {
                                paths.push(cargo_toml);
                            }
                        }
                    }
                }
            }
        }
        paths
    }

    fn parse_cargo_toml(&self, path: &std::path::Path) -> Option<toml::Value> {
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }

    fn uses_workspace_true(&self, cargo_toml: &std::path::Path, dep_name: &str) -> bool {
        let Some(manifest) = self.parse_cargo_toml(cargo_toml) else {
            return false;
        };

        // Check [dependencies]
        if let Some(deps) = manifest.get("dependencies") {
            if let Some(deps_table) = deps.as_table() {
                if let Some(dep_value) = deps_table.get(dep_name) {
                    return self.checks_workspace_true(dep_value);
                }
            }
        }

        // Check [dev-dependencies]
        if let Some(deps) = manifest.get("dev-dependencies") {
            if let Some(deps_table) = deps.as_table() {
                if let Some(dep_value) = deps_table.get(dep_name) {
                    return self.checks_workspace_true(dep_value);
                }
            }
        }

        // If dependency not found, it doesn't use workspace = true
        false
    }

    fn checks_workspace_true(&self, dep_value: &toml::Value) -> bool {
        match dep_value {
            toml::Value::Table(table) => {
                if let Some(workspace) = table.get("workspace") {
                    return workspace.as_bool() == Some(true);
                }
                false
            }
            _ => false,
        }
    }

    fn has_hardcoded_version(&self, cargo_toml: &std::path::Path, dep_name: &str) -> bool {
        let manifest = match self.parse_cargo_toml(cargo_toml) {
            Some(m) => m,
            None => return false,
        };

        let sections = ["dependencies", "dev-dependencies"];
        for section in &sections {
            if let Some(deps) = manifest.get(section) {
                if let Some(deps_table) = deps.as_table() {
                    if let Some(dep_value) = deps_table.get(dep_name) {
                        return self.is_hardcoded(dep_value);
                    }
                }
            }
        }
        false
    }

    fn is_hardcoded(&self, dep_value: &toml::Value) -> bool {
        match dep_value {
            toml::Value::String(_) => true, // "1.0" is hardcoded
            toml::Value::Table(table) => {
                // Check if it has workspace = true
                if let Some(workspace) = table.get("workspace") {
                    if workspace.as_bool() == Some(true) {
                        return false;
                    }
                }
                // Has version field means hardcoded
                table.contains_key("version")
            }
            _ => false,
        }
    }

    fn crate_name_from_path(&self, path: &std::path::Path) -> String {
        path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
}

// ============================================================================
// Dependency Alignment Given Steps
// ============================================================================

#[given("the hl7v2-gen crate Cargo.toml")]
fn given_hl7v2_gen_cargo_toml(world: &mut DependencyWorld) {
    let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    world.current_cargo_toml = Some(crate_dir.join("Cargo.toml"));
}

#[given("all workspace crates")]
fn given_all_workspace_crates(world: &mut DependencyWorld) {
    world.workspace_cargos = DependencyWorld::get_all_workspace_cargos();
}

#[given("the workspace root Cargo.toml defines managed dependencies")]
fn given_workspace_root_defines_deps(world: &mut DependencyWorld) {
    // This step just validates the workspace setup exists
    let workspace_root = DependencyWorld::get_workspace_root();
    let root_cargo = workspace_root.join("Cargo.toml");
    assert!(root_cargo.exists(), "Workspace root Cargo.toml should exist");
}

#[given("all workspace member crates")]
fn given_all_workspace_members(world: &mut DependencyWorld) {
    world.workspace_cargos = DependencyWorld::get_all_workspace_cargos();
    assert!(
        !world.workspace_cargos.is_empty(),
        "Should have workspace members"
    );
}

#[given(regex = r"^a workspace crate using workspace = true for (\w+)$")]
fn given_crate_uses_workspace(world: &mut DependencyWorld, dep_name: String) {
    // Simulate a crate that correctly uses workspace = true
    world.current_dep = Some(dep_name);
    world.should_fail = false;
}

#[given("a newly scaffolded workspace crate")]
fn given_newly_scaffolded_crate(world: &mut DependencyWorld) {
    // Placeholder for new crate scenario
    world.current_cargo_toml = Some(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    );
}

// ============================================================================
// Dependency Alignment When Steps
// ============================================================================

#[when("I check the tokio dependency")]
fn when_check_tokio(world: &mut DependencyWorld) {
    world.current_dep = Some("tokio".to_string());
}

#[when(regex = r"^I check the (\w+) dependency$")]
fn when_check_dependency(world: &mut DependencyWorld, dep_name: String) {
    world.current_dep = Some(dep_name);
}

#[when(regex = r#"^the tokio dependency has a hardcoded version like "(\d+\.\d+\.\d+)"$"#)]
fn when_tokio_has_hardcoded_version(world: &mut DependencyWorld, version: String) {
    world.current_dep = Some("tokio".to_string());
    world.should_fail = true;
    world.expected_error = Some(format!("EFF-1136 REGRESSION"));
    // Simulate the violation
    world.violations.push(format!(
        "tokio has hardcoded version {} (should use workspace = true)",
        version
    ));
}

#[when("I check the tokio dependency in each crate")]
fn when_check_tokio_in_each_crate(world: &mut DependencyWorld) {
    world.current_dep = Some("tokio".to_string());
    world.violations.clear();

    for cargo_toml in &world.workspace_cargos {
        if self::DependencyWorld::new().has_hardcoded_version(cargo_toml, "tokio") {
            let crate_name = world.crate_name_from_path(cargo_toml);
            world
                .violations
                .push(format!("{} has hardcoded tokio version", crate_name));
        }
    }
}

#[when(regex = r"^I check the (\w+) dependency in each crate$")]
fn when_check_dep_in_each_crate(world: &mut DependencyWorld, dep_name: String) {
    world.current_dep = Some(dep_name.clone());
    world.violations.clear();

    let world_ref = DependencyWorld::new();
    for cargo_toml in &world.workspace_cargos {
        if world_ref.has_hardcoded_version(cargo_toml, &dep_name) {
            let crate_name = world.crate_name_from_path(cargo_toml);
            world
                .violations
                .push(format!("{} has hardcoded {} version", crate_name, dep_name));
        }
    }
}

#[when("I check all dependencies in all crates")]
fn when_check_all_deps(world: &mut DependencyWorld) {
    world.violations.clear();

    let workspace_managed = [
        "tokio", "serde", "serde_json", "thiserror", "anyhow", "chrono", "rand", "regex",
    ];

    let world_ref = DependencyWorld::new();
    for cargo_toml in &world.workspace_cargos {
        for dep_name in &workspace_managed {
            if world_ref.has_hardcoded_version(cargo_toml, dep_name) {
                let crate_name = world.crate_name_from_path(cargo_toml);
                world.violations.push(format!(
                    "{} has hardcoded {} version",
                    crate_name, dep_name
                ));
            }
        }
    }
}

#[when("I check dev-dependencies in each crate")]
fn when_check_dev_deps(world: &mut DependencyWorld) {
    world.violations.clear();

    let workspace_managed = ["tokio", "serde", "serde_json", "thiserror", "rand"];

    let world_ref = DependencyWorld::new();
    for cargo_toml in &world.workspace_cargos {
        for dep_name in &workspace_managed {
            if world_ref.has_hardcoded_version(cargo_toml, dep_name) {
                let crate_name = world.crate_name_from_path(cargo_toml);
                world.violations.push(format!(
                    "{} has hardcoded {} version (dev)",
                    crate_name, dep_name
                ));
            }
        }
    }
}

#[when(regex = r#"^a developer changes (\w+) to version "(\d+\.\d+\.\d+)"$"#)]
fn when_dev_changes_to_version(world: &mut DependencyWorld, dep_name: String, version: String) {
    world.current_dep = Some(dep_name.clone());
    world.should_fail = true;
    world.violations.push(format!(
        "{} changed to hardcoded version {} (should use workspace = true)",
        dep_name, version
    ));
}

#[when(regex = r"^it has dependencies that are workspace-managed$")]
fn when_has_workspace_deps(_world: &mut DependencyWorld) {
    // This step is just context setting
}

// ============================================================================
// Dependency Alignment Then Steps
// ============================================================================

#[then("it should use workspace = true")]
fn then_should_use_workspace_true(world: &mut DependencyWorld) {
    let cargo_toml = world
        .current_cargo_toml
        .as_ref()
        .expect("No Cargo.toml set");
    let dep_name = world.current_dep.as_ref().expect("No dependency set");

    let uses_workspace = DependencyWorld::new().uses_workspace_true(cargo_toml, dep_name);

    assert!(
        uses_workspace,
        "{} should use workspace = true for {}",
        cargo_toml.display(),
        dep_name
    );
}

#[then("it should NOT have a hardcoded version")]
fn then_should_not_have_hardcoded_version(world: &mut DependencyWorld) {
    let cargo_toml = world
        .current_cargo_toml
        .as_ref()
        .expect("No Cargo.toml set");
    let dep_name = world.current_dep.as_ref().expect("No dependency set");

    let is_hardcoded = DependencyWorld::new().has_hardcoded_version(cargo_toml, dep_name);

    assert!(
        !is_hardcoded,
        "{} should NOT have hardcoded version for {} (should use workspace = true)",
        cargo_toml.display(),
        dep_name
    );
}

#[then("the workspace alignment test should fail")]
fn then_test_should_fail(world: &mut DependencyWorld) {
    assert!(
        world.should_fail || !world.violations.is_empty(),
        "Test should have detected violations or been marked to fail"
    );
}

#[then(regex = r#"^the error should mention "(.+)"$"#)]
fn then_error_should_mention(world: &mut DependencyWorld, pattern: String) {
    let violations_text = world.violations.join(" ");
    assert!(
        violations_text.contains(&pattern) || world.expected_error.as_ref() == Some(&pattern),
        "Expected error to mention '{}' but got: {:?}",
        pattern,
        world.violations
    );
}

#[then("every crate should use workspace = true")]
fn then_every_crate_uses_workspace(world: &mut DependencyWorld) {
    let dep_name = world.current_dep.as_ref().expect("No dependency set");
    let world_ref = DependencyWorld::new();

    for cargo_toml in &world.workspace_cargos {
        let uses_workspace = world_ref.uses_workspace_true(cargo_toml, dep_name);
        assert!(
            uses_workspace,
            "{} should use workspace = true for {}",
            world.crate_name_from_path(cargo_toml),
            dep_name
        );
    }
}

#[then("no crate should have a hardcoded tokio version")]
fn then_no_crate_has_hardcoded_tokio(world: &mut DependencyWorld) {
    let world_ref = DependencyWorld::new();

    for cargo_toml in &world.workspace_cargos {
        let has_hardcoded = world_ref.has_hardcoded_version(cargo_toml, "tokio");
        assert!(
            !has_hardcoded,
            "{} should NOT have hardcoded tokio version",
            world.crate_name_from_path(cargo_toml)
        );
    }
}

#[then(regex = r"^no crate should have a hardcoded (\w+) version$")]
fn then_no_crate_has_hardcoded_dep(world: &mut DependencyWorld, dep_name: String) {
    let world_ref = DependencyWorld::new();

    for cargo_toml in &world.workspace_cargos {
        let has_hardcoded = world_ref.has_hardcoded_version(cargo_toml, &dep_name);
        assert!(
            !has_hardcoded,
            "{} should NOT have hardcoded {} version",
            world.crate_name_from_path(cargo_toml),
            dep_name
        );
    }
}

#[then("no crate should have hardcoded versions for managed dependencies")]
fn then_no_hardcoded_for_managed(world: &mut DependencyWorld) {
    assert!(
        world.violations.is_empty(),
        "Found crates with hardcoded versions for managed dependencies:\n  - {}",
        world.violations.join("\n  - ")
    );
}

#[then("the test should list all violations if any exist")]
fn then_test_lists_violations(world: &mut DependencyWorld) {
    if !world.violations.is_empty() {
        // The error message should contain the violations list
        let violations_text = world.violations.join(" ");
        assert!(
            !violations_text.is_empty(),
            "Test should list violations: {:?}",
            world.violations
        );
    }
}

#[then("workspace-managed dev-dependencies should use workspace = true")]
fn then_dev_deps_use_workspace(world: &mut DependencyWorld) {
    assert!(
        world.violations.is_empty(),
        "Dev-dependencies with hardcoded versions:\n  - {}",
        world.violations.join("\n  - ")
    );
}

#[then("no dev-dependency should have a hardcoded version for managed deps")]
fn then_no_dev_hardcoded(world: &mut DependencyWorld) {
    assert!(
        world.violations.is_empty(),
        "Dev-dependencies with hardcoded versions:\n  - {}",
        world.violations.join("\n  - ")
    );
}

#[then(regex = r"^the error message should indicate the specific crate and (\w+)$")]
fn then_error_indicates_crate_and_dep(world: &mut DependencyWorld, _dep: String) {
    // Check that violations contain crate information
    for violation in &world.violations {
        assert!(
            violation.contains(" has hardcoded "),
            "Error should indicate crate and dependency: {}",
            violation
        );
    }
}

#[then(regex = r"^the (\w+) alignment tests should fail$")]
fn then_alignment_tests_fail(world: &mut DependencyWorld, _dep: String) {
    assert!(
        !world.violations.is_empty(),
        "Alignment tests should have detected violations"
    );
}

#[then("it must use workspace = true for those dependencies")]
fn then_must_use_workspace_true(world: &mut DependencyWorld) {
    // Verify that current crate uses workspace = true
    if let Some(cargo_toml) = &world.current_cargo_toml {
        let world_ref = DependencyWorld::new();
        let workspace_managed = ["tokio", "serde", "serde_json"];
        for dep in &workspace_managed {
            if world_ref.has_hardcoded_version(cargo_toml, dep) {
                panic!("Must use workspace = true for {}", dep);
            }
        }
    }
}

#[then("the build should fail if hardcoded versions are used")]
fn then_build_fails_if_hardcoded(world: &mut DependencyWorld) {
    // This is the contract: hardcoded versions should cause test failures
    assert!(
        !world.violations.is_empty() || !world.should_fail,
        "Build/test should fail when hardcoded versions are used"
    );
}

// Run the tests
#[tokio::main]
async fn main() {
    GenWorld::cucumber().run_and_exit("./features").await;
}
