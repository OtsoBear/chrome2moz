//! Structural validation

use crate::models::ConversionResult;
use anyhow::Result;

pub fn validate_structure(result: &ConversionResult) -> Result<()> {
    // Validate manifest
    validate_manifest(&result.manifest)?;
    
    // Validate files exist
    validate_files(result)?;
    
    Ok(())
}

fn validate_manifest(manifest: &crate::models::Manifest) -> Result<()> {
    // Check required fields
    if manifest.name.is_empty() {
        anyhow::bail!("Manifest name is required");
    }
    
    if manifest.version.is_empty() {
        anyhow::bail!("Manifest version is required");
    }
    
    if manifest.manifest_version != 3 {
        anyhow::bail!("Only Manifest V3 is supported");
    }
    
    // Check Firefox-specific requirements
    if manifest.browser_specific_settings.is_none() {
        anyhow::bail!("browser_specific_settings.gecko.id is required for Firefox");
    }
    
    Ok(())
}

fn validate_files(_result: &ConversionResult) -> Result<()> {
    // TODO: Validate that referenced files exist
    Ok(())
}