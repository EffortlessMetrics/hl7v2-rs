use crate::handlers::error::AppError;
use crate::models::{ErrorSeverity, ValidationError, ValidationWarning};

pub(super) struct ProfileValidation {
    pub(super) profile: hl7v2::Profile,
    pub(super) report: hl7v2::ValidationReport,
    pub(super) issues: Vec<hl7v2::Issue>,
}

pub(super) fn validate_message_with_profile<F>(
    message: &hl7v2::Message,
    profile_yaml: &str,
    operation: &'static str,
    profile_label: F,
) -> Result<ProfileValidation, AppError>
where
    F: FnOnce(&hl7v2::Profile) -> Option<String>,
{
    let profile = hl7v2::load_profile_checked(profile_yaml).map_err(AppError::from)?;
    let issues = hl7v2::validate(message, &profile);
    let report =
        hl7v2::ValidationReport::from_issues(message, profile_label(&profile), issues.clone());
    crate::metrics::record_validation_result(operation, report.valid);

    Ok(ProfileValidation {
        profile,
        report,
        issues,
    })
}

pub(super) fn legacy_validation_items(
    issues: Vec<hl7v2::Issue>,
) -> (Vec<ValidationError>, Vec<ValidationWarning>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for issue in issues {
        let severity = match issue.severity {
            hl7v2::Severity::Error => ErrorSeverity::Error,
            hl7v2::Severity::Warning => ErrorSeverity::Warning,
        };

        let validation_item = ValidationError {
            code: issue.code,
            message: issue.detail,
            location: issue.path,
            severity,
        };

        match issue.severity {
            hl7v2::Severity::Error => errors.push(validation_item),
            hl7v2::Severity::Warning => {
                warnings.push(ValidationWarning {
                    code: validation_item.code,
                    message: validation_item.message,
                    location: validation_item.location,
                });
            }
        }
    }

    (errors, warnings)
}
