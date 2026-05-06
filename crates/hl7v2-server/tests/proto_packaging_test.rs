#[test]
fn packaged_proto_matches_workspace_contract() {
    let workspace_proto = include_str!("../../../api/proto/hl7v2/v1/hl7v2.proto");
    let packaged_proto = include_str!("../proto/hl7v2/v1/hl7v2.proto");

    assert_eq!(
        normalize_line_endings(packaged_proto),
        normalize_line_endings(workspace_proto),
        "packaged server proto copy must match api/proto source"
    );
}

#[test]
fn packaged_openapi_matches_workspace_contract() {
    let workspace_openapi = include_str!("../../../api/openapi/hl7v2-api-v1.yaml");
    let packaged_openapi = include_str!("../openapi/hl7v2-api-v1.yaml");

    assert_eq!(
        normalize_line_endings(packaged_openapi),
        normalize_line_endings(workspace_openapi),
        "packaged server OpenAPI copy must match api/openapi source"
    );
}

fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n")
}
