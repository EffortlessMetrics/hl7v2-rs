//! Transport-level tests for `hl7v2 serve --mode grpc`.

#![expect(
    clippy::expect_used,
    reason = "integration test process cleanup and metadata setup use concise assertions"
)]

use hl7v2_server::grpc::proto::ParseRequest;
use hl7v2_server::grpc::proto::hl7_service_client::Hl7ServiceClient;
use std::error::Error;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::{Code, Request};

const SAMPLE_MSG: &[u8] = b"MSH|^~\\&|SENDING|FAC|RECV|FAC|202501010000||ADT^A01|MSG0001|P|2.5\rPID|1||123456^^^MR||Doe^John";

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child }
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            drop(self.child.kill());
        }
        drop(self.child.wait());
    }
}

#[tokio::test]
async fn cli_grpc_serve_enforces_configured_api_key() {
    let api_key = "grpc-cli-secret";
    let port = unused_local_port().expect("test should allocate a local port");
    let mut server = ChildGuard::new(
        Command::new(assert_cmd::cargo::cargo_bin("hl7v2-cli"))
            .args([
                "serve",
                "--mode",
                "grpc",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
            ])
            .env("HL7V2_API_KEY", api_key)
            .env_remove("HL7V2_CONFIG")
            .env_remove("HL7V2_CORS_ALLOWED_ORIGINS")
            .env_remove("HL7V2_PROFILE_PATHS")
            .env_remove("HL7V2_BUNDLE_OUTPUT_ROOT")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("test should spawn the canonical hl7v2 CLI binary"),
    );

    let endpoint = format!("http://127.0.0.1:{port}");
    let mut client = connect_grpc(&endpoint, &mut server)
        .await
        .expect("CLI gRPC server should accept connections");

    let missing_key_error = client
        .parse(ParseRequest {
            message: SAMPLE_MSG.to_vec(),
            mllp_framed: false,
            options: None,
        })
        .await
        .expect_err("missing CLI-configured gRPC API key should be rejected");

    assert_eq!(missing_key_error.code(), Code::Unauthenticated);
    assert!(missing_key_error.message().contains("API key"));
    assert!(!missing_key_error.message().contains(api_key));

    let mut wrong_key_request = Request::new(ParseRequest {
        message: SAMPLE_MSG.to_vec(),
        mllp_framed: false,
        options: None,
    });
    wrong_key_request.metadata_mut().insert(
        "x-api-key",
        MetadataValue::try_from("wrong-grpc-cli-secret").expect("test API key is valid"),
    );
    let wrong_key_error = client
        .parse(wrong_key_request)
        .await
        .expect_err("wrong CLI-configured gRPC API key should be rejected");

    assert_eq!(wrong_key_error.code(), Code::Unauthenticated);
    assert!(wrong_key_error.message().contains("API key"));
    assert!(!wrong_key_error.message().contains(api_key));
    assert!(!wrong_key_error.message().contains("wrong-grpc-cli-secret"));

    let mut request = Request::new(ParseRequest {
        message: SAMPLE_MSG.to_vec(),
        mllp_framed: false,
        options: None,
    });
    request.metadata_mut().insert(
        "x-api-key",
        MetadataValue::try_from(api_key).expect("test API key is valid"),
    );

    let response = client
        .parse(request)
        .await
        .expect("valid CLI-configured gRPC API key should be accepted")
        .into_inner();

    assert!(response.success);
    assert_eq!(
        response
            .metadata
            .expect("metadata should be returned")
            .message_type,
        "ADT^A01"
    );
}

#[tokio::test]
async fn cli_grpc_serve_enforces_configured_max_body_size() {
    let port = unused_local_port().expect("test should allocate a local port");
    let mut server = ChildGuard::new(
        Command::new(assert_cmd::cargo::cargo_bin("hl7v2-cli"))
            .args([
                "serve",
                "--mode",
                "grpc",
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "--max-body-size",
                "32",
            ])
            .env_remove("HL7V2_API_KEY")
            .env_remove("HL7V2_CONFIG")
            .env_remove("HL7V2_CORS_ALLOWED_ORIGINS")
            .env_remove("HL7V2_PROFILE_PATHS")
            .env_remove("HL7V2_BUNDLE_OUTPUT_ROOT")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("test should spawn the canonical hl7v2 CLI binary"),
    );

    let endpoint = format!("http://127.0.0.1:{port}");
    let mut client = connect_grpc(&endpoint, &mut server)
        .await
        .expect("CLI gRPC server should accept connections");

    let err = client
        .parse(ParseRequest {
            message: SAMPLE_MSG.to_vec(),
            mllp_framed: false,
            options: None,
        })
        .await
        .expect_err("CLI-configured oversized gRPC request should be rejected");

    assert_eq!(err.code(), Code::OutOfRange);
    assert_no_phi(err.message());
}

async fn connect_grpc(
    endpoint: &str,
    server: &mut ChildGuard,
) -> Result<Hl7ServiceClient<Channel>, Box<dyn Error>> {
    for _ in 0..80 {
        if let Some(status) = server.try_wait()? {
            return Err(
                format!("gRPC CLI server exited before accepting connections: {status}").into(),
            );
        }

        match Hl7ServiceClient::connect(endpoint.to_string()).await {
            Ok(client) => return Ok(client),
            Err(_) => sleep(Duration::from_millis(50)).await,
        }
    }

    Err(format!("gRPC CLI server did not accept connections at {endpoint}").into())
}

fn unused_local_port() -> Result<u16, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn assert_no_phi(message: &str) {
    for sentinel in ["Doe", "John", "123456", "SENDING", "MSG0001"] {
        assert!(
            !message.contains(sentinel),
            "gRPC transport error leaked sentinel {sentinel}: {message}"
        );
    }
}
