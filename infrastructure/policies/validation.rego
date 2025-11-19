# Open Policy Agent policies for HL7v2 validation and compliance

package hl7v2.validation

# Helper functions
import future.keywords.if
import future.keywords.in

# =============================================================================
# Message Structure Validation
# =============================================================================

# Deny messages without required MSH segment
deny[msg] {
    not has_msh_segment
    msg := {
        "code": "POL_MissingMSH",
        "message": "Message must start with MSH segment",
        "severity": "error"
    }
}

has_msh_segment {
    input.segments[0].id == "MSH"
}

# Deny messages without required PID segment (for ADT messages)
deny[msg] {
    is_adt_message
    not has_pid_segment
    msg := {
        "code": "POL_MissingPID",
        "message": "ADT messages must contain PID segment",
        "severity": "error"
    }
}

has_pid_segment {
    some seg in input.segments
    seg.id == "PID"
}

is_adt_message {
    startswith(input.segments[0].fields[8].value, "ADT")
}

# =============================================================================
# Message Type Validation
# =============================================================================

# Deny invalid message types
deny[msg] {
    not valid_message_type(input.segments[0].fields[8].value)
    msg := {
        "code": "POL_InvalidMessageType",
        "message": sprintf("Invalid message type: %v", [input.segments[0].fields[8].value]),
        "severity": "error",
        "allowed_types": allowed_message_types
    }
}

valid_message_type(msg_type) {
    msg_type in allowed_message_types
}

allowed_message_types := {
    "ADT^A01", "ADT^A04", "ADT^A08", "ADT^A11",
    "ORU^R01", "ORM^O01", "RDE^O11", "SIU^S12"
}

# =============================================================================
# PHI Protection
# =============================================================================

# Identify PHI fields that should be redacted in logs
phi_fields := {
    "PID.3",   # Patient ID
    "PID.5",   # Patient Name
    "PID.7",   # Date of Birth
    "PID.11",  # Patient Address
    "PID.13",  # Phone Number
    "PID.19",  # SSN
    "NK1.2",   # Next of Kin Name
    "NK1.4",   # Next of Kin Address
    "NK1.5",   # Next of Kin Phone
}

# Warn if PHI logging is enabled
warn[msg] {
    input.config.log_phi == true
    input.config.environment == "production"
    msg := {
        "code": "POL_PHILogging",
        "message": "PHI logging is enabled in production environment",
        "severity": "warning"
    }
}

# Generate redaction list for logging
redact_fields[field_path] {
    input.config.log_phi == false
    field_path in phi_fields
}

# =============================================================================
# Data Quality Rules
# =============================================================================

# Warn about suspicious ages
warn[msg] {
    some seg in input.segments
    seg.id == "PID"
    age := calculate_age(seg.fields[7].value)
    age > 120
    msg := {
        "code": "POL_SuspiciousAge",
        "message": sprintf("Age %v exceeds 120 years", [age]),
        "severity": "warning",
        "field": "PID.7"
    }
}

# Deny future birth dates
deny[msg] {
    some seg in input.segments
    seg.id == "PID"
    birth_date := seg.fields[7].value
    is_future_date(birth_date)
    msg := {
        "code": "POL_FutureBirthDate",
        "message": sprintf("Birth date %v is in the future", [birth_date]),
        "severity": "error",
        "field": "PID.7"
    }
}

# Helper: Check if date is in future
is_future_date(date_str) {
    date_str > time.now_ns()
}

# Helper: Calculate age from birth date (simplified)
calculate_age(birth_date) := age {
    # This is a simplified calculation
    # In production, use proper date parsing
    age := 50  # Placeholder
}

# =============================================================================
# Temporal Consistency Rules
# =============================================================================

# Deny if admission date is before birth date
deny[msg] {
    some pid_seg in input.segments
    some pv1_seg in input.segments
    pid_seg.id == "PID"
    pv1_seg.id == "PV1"
    birth_date := pid_seg.fields[7].value
    admit_date := pv1_seg.fields[44].value
    birth_date > admit_date
    msg := {
        "code": "POL_InvalidTemporalOrder",
        "message": "Admission date cannot be before birth date",
        "severity": "error"
    }
}

# =============================================================================
# Required Fields by Message Type
# =============================================================================

# ADT^A01 (Admit) required fields
deny[msg] {
    input.segments[0].fields[8].value == "ADT^A01"
    not has_required_adt_a01_fields
    msg := {
        "code": "POL_MissingRequiredFields",
        "message": "ADT^A01 message missing required fields",
        "severity": "error",
        "required_fields": ["PID.3", "PID.5", "PV1.2", "PV1.3"]
    }
}

has_required_adt_a01_fields {
    has_field("PID", 3)
    has_field("PID", 5)
    has_field("PV1", 2)
    has_field("PV1", 3)
}

has_field(segment_id, field_num) {
    some seg in input.segments
    seg.id == segment_id
    seg.fields[field_num].presence == "value"
}

# =============================================================================
# Table/Code Validation
# =============================================================================

# Validate gender codes (HL7 Table 0001)
deny[msg] {
    some seg in input.segments
    seg.id == "PID"
    gender := seg.fields[8].value
    not gender in valid_gender_codes
    msg := {
        "code": "POL_InvalidGenderCode",
        "message": sprintf("Invalid gender code: %v", [gender]),
        "severity": "error",
        "field": "PID.8",
        "valid_codes": valid_gender_codes
    }
}

valid_gender_codes := {"M", "F", "O", "U", "A", "N"}

# Validate patient class codes (HL7 Table 0004)
deny[msg] {
    some seg in input.segments
    seg.id == "PV1"
    patient_class := seg.fields[2].value
    not patient_class in valid_patient_class_codes
    msg := {
        "code": "POL_InvalidPatientClass",
        "message": sprintf("Invalid patient class: %v", [patient_class]),
        "severity": "error",
        "field": "PV1.2"
    }
}

valid_patient_class_codes := {"E", "I", "O", "P", "R", "B", "C", "N"}

# =============================================================================
# Compliance and Audit
# =============================================================================

# Require audit logging in production
deny[msg] {
    input.config.environment == "production"
    input.config.audit_logging == false
    msg := {
        "code": "POL_AuditRequired",
        "message": "Audit logging must be enabled in production",
        "severity": "error"
    }
}

# Require TLS in production
deny[msg] {
    input.config.environment == "production"
    input.config.tls_enabled == false
    msg := {
        "code": "POL_TLSRequired",
        "message": "TLS must be enabled in production",
        "severity": "error"
    }
}

# =============================================================================
# Rate Limiting and Quotas
# =============================================================================

# Deny if daily message quota exceeded
deny[msg] {
    input.stats.daily_message_count > input.config.max_daily_messages
    msg := {
        "code": "POL_QuotaExceeded",
        "message": sprintf("Daily message quota exceeded: %v/%v", [
            input.stats.daily_message_count,
            input.config.max_daily_messages
        ]),
        "severity": "error"
    }
}

# =============================================================================
# Summary Decision
# =============================================================================

# Allow if no denials
allow {
    count(deny) == 0
}

# Collect all warnings
warnings := warn

# Collect all denials
errors := deny

# Overall result
result := {
    "allow": allow,
    "errors": errors,
    "warnings": warnings,
    "error_count": count(errors),
    "warning_count": count(warnings)
}
