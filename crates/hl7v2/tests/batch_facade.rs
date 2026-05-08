#![cfg(feature = "batch")]

use hl7v2::batch::{BatchType, parse_batch};
use std::fmt::Debug;

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

fn require_eq<T>(actual: T, expected: T, label: &str) -> Result<(), Box<dyn std::error::Error>>
where
    T: PartialEq + Debug,
{
    if actual == expected {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("{label}: expected {expected:?}, got {actual:?}")).into())
    }
}

fn require(condition: bool, message: &'static str) -> Result<(), Box<dyn std::error::Error>> {
    if condition {
        Ok(())
    } else {
        Err(std::io::Error::other(message).into())
    }
}

#[test]
fn batch_module_parses_multi_batch_file_through_hl7v2() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"FHS|^~\\&|HIS|HOSPITAL|||20250128120000\r\
BHS|^~\\&|HIS|HOSPITAL|LAB|LABHOST|20250128120000|||LAB_BATCH\r\
MSH|^~\\&|HIS|HOSPITAL|LAB|LABHOST|20250128120100||ORM^O01|ORD001|P|2.5.1\r\
PID|1||MRN001^^^HOSP^MR||Patient^One\r\
MSH|^~\\&|HIS|HOSPITAL|LAB|LABHOST|20250128120200||ORM^O01|ORD002|P|2.5.1\r\
PID|1||MRN002^^^HOSP^MR||Patient^Two\r\
BTS|2\r\
BHS|^~\\&|HIS|HOSPITAL|RAD|RADHOST|20250128130000|||RAD_BATCH\r\
MSH|^~\\&|HIS|HOSPITAL|RAD|RADHOST|20250128130100||ORM^O01|RAD001|P|2.5.1\r\
PID|1||MRN003^^^HOSP^MR||Patient^Three\r\
BTS|1\r\
FTS|3\r";

    let batch = parse_batch(data)?;

    require_eq(batch.info.batch_type, BatchType::File, "batch type")?;
    require_eq(batch.batches.len(), 2, "nested batch count")?;
    require_eq(batch.total_message_count(), 3, "message count")?;
    require_eq(
        batch.info.message_count,
        Some(3),
        "file trailer message count",
    )?;

    Ok(())
}

#[test]
fn batch_module_accepts_messages_without_batch_headers() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120000||ADT^A01|MSG001|P|2.5.1\r\
PID|1||MRN001^^^HOSP^MR||Patient^One\r\
MSH|^~\\&|APP|FAC|RECV|RECVFAC|20250128120100||ADT^A01|MSG002|P|2.5.1\r\
PID|1||MRN002^^^HOSP^MR||Patient^Two\r";

    let batch = parse_batch(data)?;

    require_eq(batch.total_message_count(), 2, "message count")?;
    require_eq(batch.batches.len(), 1, "implicit batch count")?;

    Ok(())
}

#[test]
fn batch_module_reports_trailer_count_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let data = b"BHS|^~\\&|APP|FAC\r\
MSH|^~\\&|APP|FAC|RECV|RECVFAC|||ADT^A01|MSG|P|2.5.1\r\
BTS|5\r";

    let result = parse_batch(data);

    require(result.is_err(), "expected count mismatch")?;

    Ok(())
}
