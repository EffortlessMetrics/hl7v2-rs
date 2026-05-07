#![cfg(feature = "batch")]

use hl7v2::batch::{BatchType, parse_batch};

#[test]
fn batch_module_parses_single_batch_through_hl7v2() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"BHS|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120000|||BATCH001|Test batch\r\
MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128120001||ADT^A01|MSG001|P|2.5.1\r\
PID|1||123456^^^MRN||Doe^John\r\
BTS|1|End of batch\r";

    let batch = parse_batch(data)?;

    if batch.info.batch_type != BatchType::Single {
        return Err(std::io::Error::other(format!(
            "expected single batch, got {:?}",
            batch.info.batch_type
        ))
        .into());
    }

    if batch.total_message_count() != 1 {
        return Err(std::io::Error::other(format!(
            "expected 1 message, got {}",
            batch.total_message_count()
        ))
        .into());
    }

    Ok(())
}
