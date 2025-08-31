use std::fs::File;
use std::io::prelude::*;
use hl7v2_core::{parse, write_mllp};

fn main() -> std::io::Result<()> {
    // Create a simple HL7 message
    let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    
    // Parse the message
    let message = parse(hl7_text.as_bytes()).expect("Failed to parse HL7 message");
    
    // Wrap with MLLP framing
    let mllp_bytes = write_mllp(&message);
    
    // Write to file
    let mut file = File::create("test_mllp.hl7")?;
    file.write_all(&mllp_bytes)?;
    
    println!("Created MLLP test file with {} bytes", mllp_bytes.len());
    println!("Start byte: 0x{:02X}", mllp_bytes[0]);
    println!("End bytes: 0x{:02X} 0x{:02X}", mllp_bytes[mllp_bytes.len()-2], mllp_bytes[mllp_bytes.len()-1]);
    
    Ok(())
}