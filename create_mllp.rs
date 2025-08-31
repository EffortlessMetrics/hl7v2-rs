use std::fs::File;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    // Create a simple HL7 message with proper CR line endings
    let hl7_text = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    
    // Create MLLP framing: 0x0B + HL7 content + 0x1C 0x0D
    let mut mllp_bytes = Vec::new();
    mllp_bytes.push(0x0B); // Start byte
    mllp_bytes.extend_from_slice(hl7_text.as_bytes());
    mllp_bytes.push(0x1C); // End byte 1
    mllp_bytes.push(0x0D); // End byte 2
    
    // Write to file
    let mut file = File::create("test_mllp.hl7")?;
    file.write_all(&mllp_bytes)?;
    
    println!("Created MLLP test file with {} bytes", mllp_bytes.len());
    println!("Start byte: 0x{:02X}", mllp_bytes[0]);
    println!("End bytes: 0x{:02X} 0x{:02X}", mllp_bytes[mllp_bytes.len()-2], mllp_bytes[mllp_bytes.len()-1]);
    
    Ok(())
}