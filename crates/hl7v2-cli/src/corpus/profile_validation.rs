//! Profile-aware corpus validation counters.

use hl7v2::synthetic::corpus::{CorpusCount, CorpusFingerprintProfile, compute_sha256};
use hl7v2::{ValidationReport, is_mllp_framed, load_profile_checked, parse, parse_mllp, validate};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn fingerprint_validation_issue_counts(
    path: &Path,
    profile_path: &Path,
) -> Result<(CorpusFingerprintProfile, Vec<CorpusCount>), Box<dyn std::error::Error>> {
    let profile_yaml = fs::read_to_string(profile_path)?;
    let profile = load_profile_checked(&profile_yaml)?;
    let profile_metadata = CorpusFingerprintProfile {
        path: profile_path.to_string_lossy().to_string(),
        sha256: compute_sha256(&profile_yaml),
        version: profile.version.clone(),
        message_structure: profile.message_structure.clone(),
    };

    let mut files = Vec::new();
    collect_cli_corpus_files(path, &mut files)?;
    files.sort();

    let mut counts = std::collections::BTreeMap::new();
    for file in files {
        let bytes = fs::read(&file)?;
        let parsed = if is_mllp_framed(&bytes) {
            parse_mllp(&bytes)
        } else {
            parse(&bytes)
        };
        let Ok(message) = parsed else {
            continue;
        };
        let issues = validate(&message, &profile);
        let report = ValidationReport::from_issues(
            &message,
            Some(profile_path.to_string_lossy().to_string()),
            issues,
        );
        for issue in report.issues {
            let count = counts.entry(issue.code).or_insert(0usize);
            *count = count.saturating_add(1);
        }
    }

    Ok((profile_metadata, counts_to_corpus_counts(counts)))
}

fn collect_cli_corpus_files(
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }

    if !path.is_dir() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} is not a file or directory", path.display()),
        )));
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            collect_cli_corpus_files(&child, files)?;
        } else if child.is_file() {
            files.push(child);
        }
    }

    Ok(())
}

fn counts_to_corpus_counts(counts: std::collections::BTreeMap<String, usize>) -> Vec<CorpusCount> {
    counts
        .into_iter()
        .map(|(value, count)| CorpusCount { value, count })
        .collect()
}
