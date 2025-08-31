use std::fs;
use hl7v2_core::{parse_mllp, write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the MLLP file
    let contents = fs::read("test_mllp.hl7")?;
    println!("File size: {} bytes", contents.len());
    
    // Parse as MLLP
    let message = parse_mllp(&contents)?;
    println!("Successfully parsed MLLP message");
    
    // Write back to bytes
    let output = write(&message);
    println!("Output size: {} bytes", output.len());
    
    // Print the output as a string
    let output_str = String::from_utf8(output)?;
    println!("Output:\n{}", output_str);
    
    Ok(())
}