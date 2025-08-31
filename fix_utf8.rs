use std::fs::File;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let content = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Джон^Доу\r";
    
    let mut file = File::create("test_utf8_fixed.hl7")?;
    file.write_all(content.as_bytes())?;
    
    Ok(())
}