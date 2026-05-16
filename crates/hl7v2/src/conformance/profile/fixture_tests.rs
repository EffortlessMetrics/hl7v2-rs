#![expect(
    clippy::collapsible_if,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::manual_let_else,
    clippy::map_err_ignore,
    clippy::missing_errors_doc,
    clippy::string_slice,
    clippy::uninlined_format_args,
    reason = "Pre-existing profile implementation debt; SRP split preserves behavior."
)]

use super::*;

pub fn run_profile_fixture_tests(
    profile_path: impl AsRef<Path>,
    fixtures: impl AsRef<Path>,
    profile: &Profile,
) -> io::Result<ProfileTestReport> {
    let profile_path = profile_path.as_ref();
    let fixtures = fixtures.as_ref();
    let valid_root = fixtures.join("valid");
    let invalid_root = fixtures.join("invalid");
    let expected_root = fixtures.join("expected");
    let valid_files = collect_hl7_fixture_files(&valid_root)?;
    let invalid_files = collect_hl7_fixture_files(&invalid_root)?;
    let expected_reports =
        build_expected_report_lookup(fixtures, &expected_root, [&valid_files, &invalid_files]);

    let mut cases = Vec::new();
    cases.extend(run_profile_fixture_group(
        profile_path,
        fixtures,
        &valid_files,
        &expected_reports,
        ProfileFixtureExpectation::Valid,
        profile,
    ));
    cases.extend(run_profile_fixture_group(
        profile_path,
        fixtures,
        &invalid_files,
        &expected_reports,
        ProfileFixtureExpectation::Invalid,
        profile,
    ));

    if cases.is_empty() {
        return Err(io::Error::other(format!(
            "no .hl7 fixtures found under {}",
            fixtures.display()
        )));
    }

    let passed_count = cases.iter().filter(|case| case.passed).count();
    let case_count = cases.len();
    let failed_count = case_count.saturating_sub(passed_count);

    Ok(ProfileTestReport {
        profile: profile_path.to_string_lossy().to_string(),
        fixtures: fixtures.to_string_lossy().to_string(),
        valid: failed_count == 0,
        case_count,
        passed_count,
        failed_count,
        cases,
    })
}

fn run_profile_fixture_group(
    profile_path: &Path,
    fixture_root: &Path,
    files: &[PathBuf],
    expected_reports: &BTreeMap<PathBuf, ExpectedReportCandidate>,
    expectation: ProfileFixtureExpectation,
    profile: &Profile,
) -> Vec<ProfileTestCaseReport> {
    files
        .iter()
        .map(|path| {
            run_profile_fixture_case(
                profile_path,
                fixture_root,
                expected_reports,
                path,
                expectation,
                profile,
            )
        })
        .collect()
}

fn run_profile_fixture_case(
    profile_path: &Path,
    fixture_root: &Path,
    expected_reports: &BTreeMap<PathBuf, ExpectedReportCandidate>,
    path: &Path,
    expectation: ProfileFixtureExpectation,
    profile: &Profile,
) -> ProfileTestCaseReport {
    let name = relative_display_path(fixture_root, path);

    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(err) => {
            return ProfileTestCaseReport {
                name,
                path: path.to_string_lossy().to_string(),
                expectation,
                passed: false,
                message: format!("fixture could not be read: {err}"),
                validation_report: None,
                expected_report: None,
            };
        }
    };

    let message = match parse(&contents) {
        Ok(message) => message,
        Err(err) => {
            return ProfileTestCaseReport {
                name,
                path: path.to_string_lossy().to_string(),
                expectation,
                passed: false,
                message: format!("fixture did not parse as HL7: {err}"),
                validation_report: None,
                expected_report: None,
            };
        }
    };

    let issues = validate(&message, profile);
    let validation_report = crate::conformance::validation::ValidationReport::from_issues(
        &message,
        Some(profile_path.to_string_lossy().to_string()),
        issues,
    );
    let expected_valid = expectation == ProfileFixtureExpectation::Valid;
    let mut passed = validation_report.valid == expected_valid;
    let mut message = if passed {
        format!(
            "expected {} and report was {}",
            expectation.as_str(),
            if validation_report.valid {
                "valid"
            } else {
                "invalid"
            }
        )
    } else {
        format!(
            "expected {} but report was {}",
            expectation.as_str(),
            if validation_report.valid {
                "valid"
            } else {
                "invalid"
            }
        )
    };

    let expected_report = expected_reports
        .get(path)
        .map(|candidate| compare_expected_report_candidate(candidate, &validation_report));
    if let Some(comparison) = &expected_report {
        if comparison.matched {
            message.push_str("; expected report matched");
        } else {
            passed = false;
            let detail = comparison
                .message
                .as_deref()
                .unwrap_or("expected report did not match");
            message.push_str(&format!("; {detail}"));
        }
    }

    ProfileTestCaseReport {
        name,
        path: path.to_string_lossy().to_string(),
        expectation,
        passed,
        message,
        validation_report: Some(validation_report),
        expected_report,
    }
}

fn collect_hl7_fixture_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }

    collect_hl7_fixture_files_recursive(root, &mut files)?;
    files.sort_by(|left, right| compare_paths_case_stable(left, right));
    Ok(files)
}

fn compare_paths_case_stable(left: &Path, right: &Path) -> Ordering {
    let left_display = left.to_string_lossy();
    let right_display = right.to_string_lossy();
    left_display
        .to_lowercase()
        .cmp(&right_display.to_lowercase())
        .then_with(|| left_display.cmp(&right_display))
}

fn collect_hl7_fixture_files_recursive(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_hl7_fixture_files_recursive(&path, files)?;
        } else if file_type.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("hl7"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn build_expected_report_lookup<'a>(
    fixture_root: &Path,
    expected_root: &Path,
    fixture_groups: impl IntoIterator<Item = &'a Vec<PathBuf>>,
) -> BTreeMap<PathBuf, ExpectedReportCandidate> {
    let fixtures: Vec<&PathBuf> = fixture_groups
        .into_iter()
        .flat_map(|group| group.iter())
        .collect();
    let mut fallback_counts = BTreeMap::new();
    for fixture_path in &fixtures {
        let fallback = fallback_expected_report_path(expected_root, fixture_path);
        if fallback.exists() {
            let count = fallback_counts.entry(fallback).or_insert(0_usize);
            *count = count.saturating_add(1);
        }
    }

    let mut lookup = BTreeMap::new();
    for fixture_path in fixtures {
        let primary = primary_expected_report_path(expected_root, fixture_root, fixture_path);
        if primary.exists() {
            lookup.insert(fixture_path.clone(), ExpectedReportCandidate::File(primary));
            continue;
        }

        let fallback = fallback_expected_report_path(expected_root, fixture_path);
        match fallback_counts.get(&fallback).copied() {
            Some(1) => {
                lookup.insert(
                    fixture_path.clone(),
                    ExpectedReportCandidate::File(fallback),
                );
            }
            Some(_) => {
                lookup.insert(
                    fixture_path.clone(),
                    ExpectedReportCandidate::Ambiguous(fallback),
                );
            }
            None => {}
        }
    }
    lookup
}

fn primary_expected_report_path(
    expected_root: &Path,
    fixture_root: &Path,
    fixture_path: &Path,
) -> PathBuf {
    let relative = fixture_path
        .strip_prefix(fixture_root)
        .unwrap_or(fixture_path);
    let mut path = expected_root.join(relative);
    path.set_extension("report.json");
    path
}

fn fallback_expected_report_path(expected_root: &Path, fixture_path: &Path) -> PathBuf {
    let stem = fixture_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("fixture");
    expected_root.join(format!("{stem}.report.json"))
}

fn compare_expected_report_candidate(
    candidate: &ExpectedReportCandidate,
    actual_report: &crate::conformance::validation::ValidationReport,
) -> ExpectedReportComparison {
    match candidate {
        ExpectedReportCandidate::File(path) => {
            compare_expected_report(path, actual_report).unwrap_or_else(|| {
                ExpectedReportComparison {
                    path: path.to_string_lossy().to_string(),
                    matched: false,
                    message: Some(
                        "expected report path was registered but no longer exists".to_string(),
                    ),
                }
            })
        }
        ExpectedReportCandidate::Ambiguous(path) => ExpectedReportComparison {
            path: path.to_string_lossy().to_string(),
            matched: false,
            message: Some(
                "ambiguous basename expected report; use expected/valid/... or expected/invalid/..."
                    .to_string(),
            ),
        },
    }
}

fn compare_expected_report(
    expected_path: &Path,
    actual_report: &crate::conformance::validation::ValidationReport,
) -> Option<ExpectedReportComparison> {
    if !expected_path.exists() {
        return None;
    }

    let path = expected_path.to_string_lossy().to_string();
    let expected = match fs::read_to_string(expected_path)
        .map_err(|err| format!("expected report could not be read: {err}"))
        .and_then(|contents| {
            serde_json::from_str::<serde_json::Value>(&contents)
                .map_err(|err| format!("expected report is not valid JSON: {err}"))
        }) {
        Ok(expected) => expected,
        Err(message) => {
            return Some(ExpectedReportComparison {
                path,
                matched: false,
                message: Some(message),
            });
        }
    };

    let actual = match serde_json::to_value(actual_report) {
        Ok(actual) => actual,
        Err(err) => {
            return Some(ExpectedReportComparison {
                path,
                matched: false,
                message: Some(format!("actual report could not be serialized: {err}")),
            });
        }
    };

    match json_subset_matches(&expected, &actual, "$") {
        Ok(()) => Some(ExpectedReportComparison {
            path,
            matched: true,
            message: None,
        }),
        Err(message) => Some(ExpectedReportComparison {
            path,
            matched: false,
            message: Some(message),
        }),
    }
}

fn json_subset_matches(
    expected: &serde_json::Value,
    actual: &serde_json::Value,
    path: &str,
) -> Result<(), String> {
    match (expected, actual) {
        (serde_json::Value::Object(expected), serde_json::Value::Object(actual)) => {
            for (key, expected_value) in expected {
                let actual_value = actual
                    .get(key)
                    .ok_or_else(|| format!("{path}.{key} was missing from actual report"))?;
                json_subset_matches(expected_value, actual_value, &format!("{path}.{key}"))?;
            }
            Ok(())
        }
        (serde_json::Value::Array(expected), serde_json::Value::Array(actual)) => {
            for (index, expected_value) in expected.iter().enumerate() {
                let matched = actual.iter().any(|actual_value| {
                    json_subset_matches(expected_value, actual_value, &format!("{path}[{index}]"))
                        .is_ok()
                });
                if !matched {
                    return Err(format!(
                        "{path}[{index}] did not match any actual report item"
                    ));
                }
            }
            Ok(())
        }
        _ if expected == actual => Ok(()),
        _ => Err(format!(
            "{path} expected {} but actual report had {}",
            expected, actual
        )),
    }
}

fn relative_display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
