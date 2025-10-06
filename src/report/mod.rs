//! Report generation

pub mod generator;

use crate::models::ConversionResult;
use anyhow::Result;

pub fn generate_report(result: &ConversionResult) -> Result<String> {
    generator::generate_markdown_report(result)
}