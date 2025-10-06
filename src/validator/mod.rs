//! Validation module

pub mod structure;

use crate::models::ConversionResult;
use anyhow::Result;

pub fn validate_extension(result: &ConversionResult) -> Result<()> {
    structure::validate_structure(result)
}