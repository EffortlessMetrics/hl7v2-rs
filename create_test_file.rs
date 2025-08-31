use std::fs::File;
use std::io::Write;

fn main() {
    let content = "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\rPID|1||123456^^^HOSP^MR||Doe^John\r";
    let mut file = File::create("test.hl7").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    println!("Created test.hl7 file");
}