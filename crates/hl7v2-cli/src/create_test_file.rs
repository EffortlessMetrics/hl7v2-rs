use std::fs::File;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let mut file = File::create("test_data/correct_test.hl7")?;
    
    // Write the HL7 message with correct CR line endings
    let content = b"MSH|^~\\&|TestApp|TestFac|RecvApp|RecvFac|20250101000000||ADT^A01^ADT_A01|MSG001|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S|||123456789|\rPV1|1|O|OP^PAREG^CHAREG|3|||DOE^JOHN^A^III^^^^MD|^DR.^JANE^B^^^^RN|||SURG||||ADM|||||20250101000000\r";
    
    file.write_all(content)?;
    println!("Created test file with correct HL7 line endings");
    Ok(())
}