//! CLI error classification and process exit codes.

use std::fmt;

const EXIT_CHECK_FAILED: i32 = 1;
const EXIT_INPUT_ERROR: i32 = 2;
const EXIT_RUNTIME_ERROR: i32 = 3;

#[derive(Debug)]
pub(crate) struct CliFailure {
    code: i32,
    message: String,
}

impl CliFailure {
    pub(crate) fn check_failed(message: impl Into<String>) -> Box<dyn std::error::Error> {
        Box::new(Self {
            code: EXIT_CHECK_FAILED,
            message: message.into(),
        })
    }
}

impl fmt::Display for CliFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CliFailure {}

pub(crate) fn classify_cli_error(error: &(dyn std::error::Error + 'static)) -> i32 {
    if let Some(failure) = error.downcast_ref::<CliFailure>() {
        failure.code
    } else if let Some(error) = error.downcast_ref::<std::io::Error>() {
        match error.kind() {
            std::io::ErrorKind::InvalidInput | std::io::ErrorKind::Other => EXIT_INPUT_ERROR,
            _ => EXIT_RUNTIME_ERROR,
        }
    } else {
        EXIT_INPUT_ERROR
    }
}
