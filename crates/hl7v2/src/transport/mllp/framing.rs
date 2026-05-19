use crate::model::Error;

use super::constants::{MLLP_END_1, MLLP_END_2, MLLP_START};
use super::errors::MllpError;

pub fn wrap_mllp(bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(bytes.len() + 3);
    buf.push(MLLP_START);
    buf.extend_from_slice(bytes);
    buf.push(MLLP_END_1);
    buf.push(MLLP_END_2);
    buf
}

pub fn unwrap_mllp(bytes: &[u8]) -> Result<&[u8], Error> {
    if bytes.is_empty() || bytes[0] != MLLP_START {
        return Err(Error::Framing(
            "Missing MLLP start block character (0x0B)".to_string(),
        ));
    }

    let end_pos = find_mllp_end(bytes)?;
    Ok(&bytes[1..end_pos])
}

pub fn unwrap_mllp_checked(bytes: &[u8]) -> Result<&[u8], MllpError> {
    if bytes.is_empty() || bytes[0] != MLLP_START {
        return Err(MllpError::MissingStartBlock);
    }

    let end_pos = find_mllp_end_checked(bytes)?;
    Ok(&bytes[1..end_pos])
}

pub fn unwrap_mllp_owned(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    unwrap_mllp(bytes).map(<[u8]>::to_vec)
}

pub fn unwrap_mllp_owned_checked(bytes: &[u8]) -> Result<Vec<u8>, MllpError> {
    unwrap_mllp_checked(bytes).map(<[u8]>::to_vec)
}

fn find_mllp_end(bytes: &[u8]) -> Result<usize, Error> {
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == MLLP_END_1 && bytes[i + 1] == MLLP_END_2 {
            return Ok(i);
        }
    }
    Err(Error::Framing(
        "Missing MLLP end block sequence (0x1C 0x0D)".to_string(),
    ))
}

fn find_mllp_end_checked(bytes: &[u8]) -> Result<usize, MllpError> {
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == MLLP_END_1 && bytes[i + 1] == MLLP_END_2 {
            return Ok(i);
        }
    }
    Err(MllpError::MissingEndBlock)
}

pub fn is_mllp_framed(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes[0] == MLLP_START
}

pub fn find_complete_mllp_message(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() || bytes[0] != MLLP_START {
        return None;
    }

    for i in 1..bytes.len().saturating_sub(1) {
        if bytes[i] == MLLP_END_1 && bytes[i + 1] == MLLP_END_2 {
            return Some(i + 2);
        }
    }

    None
}
