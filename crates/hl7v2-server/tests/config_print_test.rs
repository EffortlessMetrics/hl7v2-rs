//! Tests for the standalone server configuration print mode.

#[test]
fn print_config_outputs_sanitized_effective_config_and_exits()
-> Result<(), Box<dyn std::error::Error>> {
    let output = assert_cmd::Command::cargo_bin("hl7v2-server")?
        .arg("--print-config")
        .env("BIND_ADDRESS", "127.0.0.1:19090")
        .env("HL7V2_API_KEY", "super-secret")
        .env("HL7V2_CORS_ALLOWED_ORIGINS", "https://app.example")
        .env_remove("HL7V2_CONFIG")
        .env_remove("HL7V2_MAX_MESSAGE_SIZE")
        .env_remove("HL7V2_PROFILE_PATHS")
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "print-config exited with status {}",
            output.status
        ))
        .into());
    }
    if !output.stderr.is_empty() {
        return Err(std::io::Error::other("print-config should not write stderr").into());
    }
    let stdout = String::from_utf8(output.stdout)?;
    if stdout.contains("super-secret") {
        return Err(std::io::Error::other("print-config leaked the API key value").into());
    }

    let config: serde_json::Value = serde_json::from_str(&stdout)?;
    if config
        .get("bind_address")
        .and_then(serde_json::Value::as_str)
        != Some("127.0.0.1:19090")
    {
        return Err(std::io::Error::other("print-config did not include bind address").into());
    }
    if config
        .get("api_key_configured")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Err(std::io::Error::other("print-config did not mark API key configured").into());
    }
    if config
        .get("max_message_size")
        .and_then(serde_json::Value::as_u64)
        != Some(50 * 1024 * 1024)
    {
        return Err(std::io::Error::other("print-config did not include max message size").into());
    }
    let cors = config.get("cors_allowed_origins");
    if cors
        .and_then(|value| value.get("mode"))
        .and_then(serde_json::Value::as_str)
        != Some("list")
    {
        return Err(std::io::Error::other("print-config did not include CORS list mode").into());
    }
    if cors
        .and_then(|value| value.get("origins"))
        .and_then(serde_json::Value::as_array)
        .and_then(|origins| origins.first())
        .and_then(serde_json::Value::as_str)
        != Some("https://app.example")
    {
        return Err(std::io::Error::other("print-config did not include CORS origin").into());
    }

    Ok(())
}
