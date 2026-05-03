extern crate tonic_build;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../../api/proto/hl7v2.proto"], &["../../api/proto"])?;
    Ok(())
}
