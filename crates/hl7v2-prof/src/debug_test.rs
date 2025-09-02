#[cfg(test)]
mod tests {
    use crate::{parse_hl7_ts_with_precision, compare_timestamps_for_before, ParsedTimestamp, TimestampPrecision};
    use chrono::NaiveDate;

    #[test]
    fn test_compare_same_dates() {
        let date_str = "20241201";
        let ts1 = parse_hl7_ts_with_precision(date_str).unwrap();
        let ts2 = parse_hl7_ts_with_precision(date_str).unwrap();
        
        println!("ts1: {:?}, ts2: {:?}", ts1, ts2);
        
        let result = compare_timestamps_for_before(&ts1, &ts2);
        println!("compare_timestamps_for_before result: {}", result);
        
        // This should be false because they're equal
        assert!(!result, "Expected false for equal dates, but got true");
    }
}