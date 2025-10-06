//! Manifest data structures for Chrome and Firefox extensions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub manifest_version: u8,
    pub name: String,
    pub version: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<Action>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_action: Option<Action>,
    
    #[serde(default)]
    pub permissions: Vec<String>,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub host_permissions: Vec<String>,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_scripts: Vec<ContentScript>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_accessible_resources: Option<WebAccessibleResources>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_security_policy: Option<ContentSecurityPolicy>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_specific_settings: Option<BrowserSpecificSettings>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icons: Option<HashMap<String, String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<HashMap<String, Command>>,
    
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Background {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_worker: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scripts: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_popup: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_icon: Option<IconSet>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_title: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_style: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IconSet {
    Single(String),
    Multiple(HashMap<String, String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentScript {
    pub matches: Vec<String>,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub js: Vec<String>,
    
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub css: Vec<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_at: Option<String>,
    
    #[serde(default)]
    pub all_frames: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WebAccessibleResources {
    V2(Vec<String>),
    V3(Vec<WebAccessibleResourceV3>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAccessibleResourceV3 {
    pub resources: Vec<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matches: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_ids: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_dynamic_url: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentSecurityPolicy {
    V2(String),
    V3(ContentSecurityPolicyV3),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSecurityPolicyV3 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension_pages: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSpecificSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gecko: Option<GeckoSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeckoSettings {
    pub id: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_min_version: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_max_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_key: Option<HashMap<String, String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}