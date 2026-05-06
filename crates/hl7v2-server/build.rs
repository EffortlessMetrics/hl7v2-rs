//! Build helper for generating gRPC bindings during compilation.

extern crate tonic_build;

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;

    // SAFETY: build.rs runs before compiling this crate. We set PROTOC once
    // for prost/tonic codegen and do not spawn concurrent Rust threads here.
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let workspace_proto_dir = manifest_dir.join("../../api/proto");
    let workspace_proto = workspace_proto_dir.join("hl7v2/v1/hl7v2.proto");
    let packaged_proto_dir = manifest_dir.join("proto");
    let packaged_proto = packaged_proto_dir.join("hl7v2/v1/hl7v2.proto");
    let (proto_dir, proto_file) = if workspace_proto.exists() {
        (workspace_proto_dir, workspace_proto)
    } else {
        (packaged_proto_dir, packaged_proto)
    };

    let workspace_openapi = manifest_dir.join("../../api/openapi/hl7v2-api-v1.yaml");
    let packaged_openapi = manifest_dir.join("openapi/hl7v2-api-v1.yaml");
    let openapi_yaml = if workspace_openapi.exists() {
        workspace_openapi
    } else {
        packaged_openapi
    };

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&[proto_file.as_path()], &[proto_dir.as_path()])?;

    println!(
        "cargo:rustc-env=HL7V2_OPENAPI_YAML={}",
        openapi_yaml.display()
    );
    println!("cargo:rerun-if-changed={}", proto_file.display());
    println!("cargo:rerun-if-changed={}", proto_dir.display());
    println!("cargo:rerun-if-changed={}", openapi_yaml.display());

    Ok(())
}
