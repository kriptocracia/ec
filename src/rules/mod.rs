use std::path::Path;

use anyhow::{Context, Result};

pub use self::types::*;

mod types;

/// Load and parse a rule file by its id.
/// Looks in: {rules_dir}/{rules_id}.toml
pub fn load_rules(rules_id: &str, rules_dir: &Path) -> Result<ElectionRules> {
    let path = rules_dir.join(format!("{rules_id}.toml"));
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Rules file not found: {}", path.display()))?;
    let rules: ElectionRules = toml::from_str(&content)
        .with_context(|| format!("Failed to parse rules file: {}", path.display()))?;
    Ok(rules)
}
