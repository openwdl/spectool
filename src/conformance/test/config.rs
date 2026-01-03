//! Configuration for conformance tests.

use serde::Deserialize;
use serde::Serialize;
use strum_macros::EnumIter;

/// A tag associated with a conformance test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tag {
    /// Test is for deprecated functionality.
    Deprecated,
}

/// A capability required by a conformance test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Requires specific CPU resources.
    Cpu,
    /// Requires specific memory resources.
    Memory,
    /// Requires GPU hardware.
    Gpu,
    /// Requires specific disk resources.
    Disks,
    /// Allows setting nested workflow/task inputs at runtime.
    #[value(name = "allow_nested_inputs")]
    AllowNestedInputs,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::Cpu => write!(f, "cpu"),
            Capability::Memory => write!(f, "memory"),
            Capability::Gpu => write!(f, "gpu"),
            Capability::Disks => write!(f, "disks"),
            Capability::AllowNestedInputs => write!(f, "allow_nested_inputs"),
        }
    }
}

/// The expected return code(s) for a conformance test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[derive(Default)]
pub enum ReturnCode {
    /// Any return code is allowed.
    #[serde(deserialize_with = "deserialize_any")]
    #[default]
    Any,
    /// A single expected return code.
    Single(i32),
    /// Multiple possible return codes.
    Multiple(Vec<i32>),
}

/// Custom deserializer for the "*" string to represent Any.
fn deserialize_any<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s == "*" {
        Ok(())
    } else {
        Err(serde::de::Error::custom("expected \"*\""))
    }
}

/// A configuration for a conformance test.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The target workflow or task to execute.
    ///
    /// This field may ONLY be specified when there are multiple tasks and no input JSON.
    /// It MUST NOT be specified when the target can be inferred.
    target: Option<String>,

    /// Whether to skip this test entirely.
    #[serde(default)]
    ignore: bool,

    /// Whether the test is expected to fail.
    #[serde(default)]
    fail: bool,

    /// The expected return code(s).
    #[serde(default)]
    return_code: ReturnCode,

    /// Output keys to ignore when validating.
    #[serde(default)]
    exclude_outputs: Vec<String>,

    /// Runtime capabilities required by the test.
    #[serde(default)]
    capabilities: Vec<Capability>,

    /// Tags associated with the test (e.g., deprecated).
    #[serde(default)]
    tags: Vec<Tag>,
}

impl Config {
    /// Gets the target workflow or task name.
    pub fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    /// Returns whether this test should be ignored.
    pub fn ignore(&self) -> bool {
        self.ignore
    }

    /// Returns whether this test is expected to fail.
    pub fn fail(&self) -> bool {
        self.fail
    }

    /// Gets the expected return code(s).
    pub fn return_code(&self) -> &ReturnCode {
        &self.return_code
    }

    /// Gets the output keys to exclude from validation.
    pub fn exclude_outputs(&self) -> &[String] {
        &self.exclude_outputs
    }

    /// Gets the required capabilities.
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Gets the tags associated with the test.
    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let json = "{}";
        let config: Config = serde_json::from_str(json).unwrap();

        assert_eq!(config.target(), None);
        assert!(!config.ignore());
        assert!(!config.fail());
        assert_eq!(config.return_code(), &ReturnCode::Any);
        assert_eq!(config.exclude_outputs(), &[] as &[String]);
        assert_eq!(config.capabilities(), &[] as &[Capability]);
        assert_eq!(config.tags(), &[] as &[Tag]);
    }

    #[test]
    fn return_code_any() {
        let json = r#"{"return_code": "*"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.return_code(), &ReturnCode::Any);
    }

    #[test]
    fn return_code_single() {
        let json = r#"{"return_code": 1}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.return_code(), &ReturnCode::Single(1));
    }

    #[test]
    fn return_code_multiple() {
        let json = r#"{"return_code": [1, 2, 3]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.return_code(), &ReturnCode::Multiple(vec![1, 2, 3]));
    }

    #[test]
    fn capabilities() {
        let json = r#"{"capabilities": ["gpu", "memory"]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.capabilities(),
            &[Capability::Gpu, Capability::Memory]
        );
    }

    #[test]
    fn full_config() {
        let json = r#"{
            "target": "my_task",
            "ignore": true,
            "fail": true,
            "return_code": 1,
            "exclude_outputs": ["timestamp"],
            "capabilities": ["cpu", "gpu"],
            "tags": ["deprecated"]
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert_eq!(config.target(), Some("my_task"));
        assert!(config.ignore());
        assert!(config.fail());
        assert_eq!(config.return_code(), &ReturnCode::Single(1));
        assert_eq!(config.exclude_outputs(), &["timestamp"]);
        assert_eq!(config.capabilities(), &[Capability::Cpu, Capability::Gpu]);
        assert_eq!(config.tags(), &[Tag::Deprecated]);
    }

    #[test]
    fn tags() {
        let json = r#"{"tags": ["deprecated"]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.tags(), &[Tag::Deprecated]);
    }

    #[test]
    fn unknown_field_rejected() {
        let json = r#"{"unknown_field": "value"}"#;
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn unknown_capability_rejected() {
        let json = r#"{"capabilities": ["unknown"]}"#;
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
