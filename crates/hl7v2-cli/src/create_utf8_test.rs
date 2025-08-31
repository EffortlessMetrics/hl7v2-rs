use std::fs::File;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let mut file = File::create("test_data/ascii_test.hl7")?;
    
    // Write the HL7 message with correct CR line endings
    let content = b"MSH|^~\\&|TestApp|TestFac|RecvApp|RecvFac|20250101000000||ADT^A01^ADT_A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\r";
    
    file.write_all(content)?;
    println!("Created ASCII test file with correct HL7 line endings");
    
    // Create a UTF-8 test file with Cyrillic characters
    let mut utf8_file = File::create("test_data/utf8_test.hl7")?;
    let utf8_content = b"MSH|^~\\&|\xD0\xA2\xD0\xB5\xD1\x81\xD1\x82App|\xD0\xA2\xD0\xB5\xD1\x81\xD1\x82Fac|RecvApp|RecvFac|20250101000000||ADT^A01^ADT_A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||\xD0\x94\xD0\xBE\xD0\xB5^\xD0\x94\xD0\xBE\xD1\x83||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\r";
    
    utf8_file.write_all(utf8_content)?;
    println!("Created UTF-8 test file with Cyrillic characters");
    
    Ok(())
}