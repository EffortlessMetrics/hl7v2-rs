//! HL7 v2 MLLP Client Example
//!
//! This example demonstrates how to:
//! - Connect to an MLLP server using the hl7v2-network crate
//! - Send an HL7 message and receive an ACK response
//! - Handle timeouts and connection errors
//!
//! Note: This example requires a running MLLP server.
//! You can start a test server with: cargo run --package hl7v2-cli -- serve --port 2575
//!
//! Run with: cargo run --example mllp_client

use hl7v2_core::{parse, write, Message, get};
use hl7v2_network::{MllpClient, MllpClientBuilder};
use std::time::Duration;

/// Default MLLP server address
const DEFAULT_SERVER: &str = "127.0.0.1:2575";

/// Sample ADT^A01 message to send
const SAMPLE_ADT_A01: &[u8] = b"MSH|^~\\&|HL7V2RS|HOSPITAL|LABSYSTEM|LABORATORY|20250128152312||ADT^A01^ADT_A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||DOE^JOHN||19850315|M\rPV1|1|I|3N^301^A||||DR123^SMITH^JOHN^^^MD||||||||ADM|A0|||||||||||||||||||HOSPITAL||20250128120000|||\r";

#[tokio::main]
async fn main() {
    println!("=== HL7 v2 MLLP Client Example ===\n");

    // Parse command line arguments for server address
    let server_addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_SERVER.to_string());

    println!("Target server: {}", server_addr);
    println!();

    // Example 1: Basic client with default configuration
    match send_message_basic(&server_addr).await {
        Ok(ack) => {
            println!("✓ Received ACK response\n");
            display_ack_details(&ack);
        }
        Err(e) => {
            eprintln!("✗ Failed to send message: {}", e);
            eprintln!("\n  Hint: Make sure an MLLP server is running at {}", server_addr);
            eprintln!("  You can start a test server with:");
            eprintln!("    cargo run --package hl7v2-cli -- serve --port 2575");
        }
    }

    println!();

    // Example 2: Client with custom configuration
    println!("--- Example 2: Custom Configuration ---\n");
    let config_info = r#"
Client configuration:
    MllpClientBuilder::new()
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(10))
        .write_timeout(Duration::from_secs(10))
        .max_frame_size(10 * 1024 * 1024)  // 10MB
        .build()
"#;
    println!("{}", config_info);
    println!();

    // Example 3: Error handling patterns
    println!("--- Example 3: Error Handling Patterns ---\n");
    demonstrate_error_handling().await;
}

/// Send a message using basic client configuration
async fn send_message_basic(server_addr: &str) -> Result<Message, Box<dyn std::error::Error>> {
    println!("--- Example 1: Basic Message Exchange ---\n");

    // Create a client with custom configuration
    let mut client = MllpClientBuilder::new()
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(30))
        .write_timeout(Duration::from_secs(30))
        .build();

    // Connect to the server
    println!("Connecting to {}...", server_addr);
    let addr: std::net::SocketAddr = server_addr.parse()?;
    client.connect(addr).await?;
    println!("✓ Connected successfully\n");

    // Parse the sample message
    let message = parse(SAMPLE_ADT_A01)?;
    
    println!("Sending ADT^A01 message...");
    println!("  Message Control ID: {:?}", get(&message, "MSH.10"));
    println!("  Patient: {:?}", get(&message, "PID.5"));
    println!();

    // Send the message and wait for ACK
    let ack = client.send_message(&message).await?;

    // Close the connection
    client.close().await?;

    Ok(ack)
}

/// Display details of an ACK response
fn display_ack_details(ack: &Message) {
    println!("ACK Message Details:");
    
    // Get MSA segment fields
    let ack_code = get(ack, "MSA.1");
    let message_control = get(ack, "MSA.2");
    let text_message = get(ack, "MSA.3");

    println!("  MSA-1 (Acknowledgment Code): {:?}", ack_code);
    println!("  MSA-2 (Message Control ID): {:?}", message_control);
    println!("  MSA-3 (Text Message): {:?}", text_message);
    println!();

    // Interpret the acknowledgment code
    if let Some(code) = ack_code {
        match code {
            s if s == "AA" => println!("  Status: ✓ Application Accept - Message processed successfully"),
            s if s == "AE" => println!("  Status: ✗ Application Error - Processing failed"),
            s if s == "AR" => println!("  Status: ✗ Application Reject - Message rejected"),
            s if s == "CA" => println!("  Status: ✓ Commit Accept"),
            s if s == "CE" => println!("  Status: ✗ Commit Error"),
            s if s == "CR" => println!("  Status: ✗ Commit Reject"),
            _ => println!("  Status: ? Unknown acknowledgment code: {}", code),
        }
    }
    println!();

    // Display the full ACK message
    let ack_bytes = write(ack);
    println!("Full ACK message:");
    println!("{}", String::from_utf8_lossy(&ack_bytes).replace("\r", "\r\n"));
}

/// Demonstrate error handling patterns
async fn demonstrate_error_handling() {
    // Pattern 1: Connection timeout
    println!("Pattern 1: Handling connection timeouts");
    println!("  When connecting to an unavailable server:");
    println!("  ```rust");
    println!("  let mut client = MllpClientBuilder::new()");
    println!("      .connect_timeout(Duration::from_secs(5))");
    println!("      .build();");
    println!("  match client.connect(addr).await {{");
    println!("      Ok(()) => println!(\"Connected\"),");
    println!("      Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {{");
    println!("          eprintln!(\"Connection timed out\");");
    println!("      }}");
    println!("      Err(e) => eprintln!(\"Connection failed: {{}}\", e),");
    println!("  }}");
    println!("  ```");
    println!();

    // Pattern 2: Read timeout (no response)
    println!("Pattern 2: Handling read timeouts (no ACK received)");
    println!("  ```rust");
    println!("  match client.send_message(&message).await {{");
    println!("      Ok(ack) => process_ack(ack),");
    println!("      Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {{");
    println!("          eprintln!(\"Server did not respond within timeout\");");
    println!("          // Consider retry logic here");
    println!("      }}");
    println!("      Err(e) => eprintln!(\"Send failed: {{}}\", e),");
    println!("  }}");
    println!("  ```");
    println!();

    // Pattern 3: Invalid response
    println!("Pattern 3: Handling invalid ACK responses");
    println!("  ```rust");
    println!("  match client.send_message(&message).await {{");
    println!("      Ok(ack) => {{");
    println!("          // Verify ACK structure");
    println!("          let ack_code = get(&ack, \"MSA.1\");");
    println!("          match ack_code.as_deref() {{");
    println!("              Some(\"AA\") | Some(\"CA\") => println!(\"Success\"),");
    println!("              Some(\"AE\") | Some(\"CE\") => println!(\"Error: {{:?}}\", get(&ack, \"MSA.3\")),");
    println!("              Some(\"AR\") | Some(\"CR\") => println!(\"Rejected\"),");
    println!("              _ => eprintln!(\"Invalid ACK code: {{:?}}\", ack_code),");
    println!("          }}");
    println!("      }}");
    println!("      Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {{");
    println!("          eprintln!(\"Server returned invalid HL7 data\");");
    println!("      }}");
    println!("      Err(e) => eprintln!(\"Error: {{}}\", e),");
    println!("  }}");
    println!("  ```");
    println!();

    // Pattern 4: Connection lost
    println!("Pattern 4: Handling connection loss");
    println!("  ```rust");
    println!("  match client.send_message(&message).await {{");
    println!("      Ok(ack) => process_ack(ack),");
    println!("      Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {{");
    println!("          eprintln!(\"Connection closed by server\");");
    println!("          // Attempt reconnection");
    println!("          client.connect(addr).await?;");
    println!("      }}");
    println!("      Err(e) => return Err(e.into()),");
    println!("  }}");
    println!("  ```");
    println!();

    // Pattern 5: Retry with exponential backoff
    println!("Pattern 5: Retry with exponential backoff");
    println!("  ```rust");
    println!("  async fn send_with_retry(client: &mut MllpClient, message: &Message, max_retries: u32) {{");
    println!("      let mut delay = Duration::from_millis(100);");
    println!("      for attempt in 1..=max_retries {{");
    println!("          match client.send_message(message).await {{");
    println!("              Ok(ack) => return Ok(ack),");
    println!("              Err(e) if attempt < max_retries => {{");
    println!("                  tokio::time::sleep(delay).await;");
    println!("                  delay *= 2; // Exponential backoff");
    println!("              }}");
    println!("              Err(e) => return Err(e),");
    println!("          }}");
    println!("      }}");
    println!("  }}");
    println!("  ```");
    println!();
}
