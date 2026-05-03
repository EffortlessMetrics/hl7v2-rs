extern crate tonic_build;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;

    // SAFETY: build.rs runs before compiling this crate. We set PROTOC once
    // for prost/tonic codegen and do not spawn concurrent Rust threads here.
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../../api/proto/hl7v2.proto"], &["../../api/proto"])?;

    println!("cargo:rerun-if-changed=../../api/proto/hl7v2.proto");
    println!("cargo:rerun-if-changed=../../api/proto");

    Ok(())
}
