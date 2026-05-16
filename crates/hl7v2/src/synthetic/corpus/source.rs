//! Corpus input discovery and parsing helpers.
//!
//! This module owns filesystem traversal, display path normalization, and
//! plain-vs-MLLP parse selection for corpus inputs.

use super::CorpusError;
use crate::model::{Error, Message};
use crate::parser::{parse, parse_mllp};
use crate::transport::mllp::is_mllp_framed;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn collect_corpus_files(
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), CorpusError> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }

    if !path.is_dir() {
        return Err(CorpusError::InvalidConfig(format!(
            "{} is not a file or directory",
            path.display()
        )));
    }

    for entry in fs::read_dir(path).map_err(|e| CorpusError::IoError(e.to_string()))? {
        let entry = entry.map_err(|e| CorpusError::IoError(e.to_string()))?;
        let child = entry.path();
        if child.is_dir() {
            collect_corpus_files(&child, files)?;
        } else if child.is_file() {
            files.push(child);
        }
    }

    Ok(())
}

pub(super) fn parse_corpus_message_bytes(message_bytes: &[u8]) -> Result<Message, Error> {
    if is_mllp_framed(message_bytes) {
        parse_mllp(message_bytes)
    } else {
        parse(message_bytes)
    }
}

pub(super) fn relative_corpus_path(root: &Path, file: &Path) -> String {
    let relative = if root.is_dir() {
        file.strip_prefix(root).unwrap_or(file)
    } else {
        file.file_name().map(Path::new).unwrap_or(file)
    };
    relative.to_string_lossy().replace('\\', "/")
}
