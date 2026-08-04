//! Optional parser observability hooks.
//!
//! Metrics are deliberately emitted only at public parser entry points. Error
//! labels use a fixed vocabulary so malformed input cannot create unbounded
//! metric cardinality or leak message content.

#![cfg(feature = "metrics")]

use std::time::Instant;

use crate::model::Error;

pub(crate) struct ParseObservation {
    operation: &'static str,
    input_bytes: usize,
    started: Instant,
}

impl ParseObservation {
    pub(crate) fn start(operation: &'static str, input_bytes: usize) -> Self {
        Self {
            operation,
            input_bytes,
            started: Instant::now(),
        }
    }

    pub(crate) fn finish<T>(self, result: &Result<T, Error>, item_count: Option<usize>) {
        metrics::counter!(
            "hl7v2_parser_parse_total",
            "operation" => self.operation,
            "outcome" => outcome(result)
        )
        .increment(1);

        metrics::histogram!(
            "hl7v2_parser_parse_duration_seconds",
            "operation" => self.operation
        )
        .record(self.started.elapsed().as_secs_f64());
        #[expect(
            clippy::cast_precision_loss,
            reason = "metrics observations use f64 and retain the full count range approximately"
        )]
        metrics::histogram!(
            "hl7v2_parser_parse_input_bytes",
            "operation" => self.operation
        )
        .record(self.input_bytes as f64);

        if let Some(item_count) = item_count {
            #[expect(
                clippy::cast_precision_loss,
                reason = "metrics observations use f64 and retain the full count range approximately"
            )]
            metrics::histogram!(
                "hl7v2_parser_parse_items",
                "operation" => self.operation
            )
            .record(item_count as f64);
        }

        if let Err(error) = result {
            metrics::counter!(
                "hl7v2_parser_parse_errors_total",
                "operation" => self.operation,
                "kind" => error_kind(error)
            )
            .increment(1);
        }
    }
}

fn outcome<T>(result: &Result<T, Error>) -> &'static str {
    if result.is_ok() { "success" } else { "error" }
}

fn error_kind(error: &Error) -> &'static str {
    match error {
        Error::InvalidSegmentId => "invalid_segment_id",
        Error::BadDelimLength => "bad_delimiter_length",
        Error::DuplicateDelims => "duplicate_delimiters",
        Error::UnbalancedEscape => "unbalanced_escape",
        Error::InvalidEscapeToken => "invalid_escape_token",
        Error::MshFieldMalformed => "msh_field_malformed",
        Error::Msh10Missing => "msh10_missing",
        Error::InvalidProcessingId => "invalid_processing_id",
        Error::UnrecognizedVersion => "unrecognized_version",
        Error::InvalidCharset => "invalid_charset",
        Error::Framing(_) => "framing",
        Error::WriteFailed => "write_failed",
        Error::ParseError { .. } => "parse_error",
        Error::InvalidFieldFormat { .. } => "invalid_field_format",
        Error::InvalidRepFormat { .. } => "invalid_repetition_format",
        Error::InvalidCompFormat { .. } => "invalid_component_format",
        Error::InvalidSubcompFormat { .. } => "invalid_subcomponent_format",
        Error::BatchParseError { .. } => "batch_parse_error",
        Error::InvalidBatchHeader { .. } => "invalid_batch_header",
        Error::InvalidBatchTrailer { .. } => "invalid_batch_trailer",
    }
}

#[cfg(test)]
mod tests {
    use super::{error_kind, outcome};
    use crate::model::Error;

    #[test]
    fn error_labels_are_static_and_non_sensitive() {
        assert_eq!(
            error_kind(&Error::InvalidFieldFormat {
                details: "patient identifier".to_string(),
            }),
            "invalid_field_format"
        );
        assert_eq!(
            error_kind(&Error::Framing("secret payload".to_string())),
            "framing"
        );
    }

    #[test]
    fn parse_outcome_has_a_fixed_vocabulary() {
        assert_eq!(outcome::<()>(&Ok(())), "success");
        assert_eq!(outcome::<()>(&Err(Error::InvalidSegmentId)), "error");
    }
}
