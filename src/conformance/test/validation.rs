//! Validation of conformance test results.

use std::borrow::Cow;
use std::path::Path;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use serde_json::Value;

/// Validates that the actual output matches the expected output.
///
/// This function performs a deep comparison of JSON values, excluding any
/// keys specified in the `exclude` list.
///
/// # Arguments
///
/// * `expected` - The expected output value from the test specification
/// * `actual` - The actual output value from the test execution
/// * `exclude` - A list of output keys to exclude from validation
///
/// # Returns
///
/// Returns `Ok(())` if the outputs match, or an error with details about the mismatch.
pub fn validate_outputs(expected: &Value, actual: &Value, exclude: &[String]) -> Result<()> {
    let expected_filtered = filter_outputs(expected, exclude);
    let actual_filtered = filter_outputs(actual, exclude);

    compare_json(&expected_filtered, &actual_filtered, "")
}

/// Filters out excluded keys from a JSON value.
///
/// This function recursively processes JSON objects and removes any keys
/// that are in the exclude list. Supports both simple keys ("timestamp") and
/// nested paths ("nested.timestamp").
fn filter_outputs(value: &Value, exclude: &[String]) -> Value {
    filter_outputs_recursive(value, exclude, "")
}

/// Recursively filters outputs with path tracking for dot notation support.
fn filter_outputs_recursive(value: &Value, exclude: &[String], current_path: &str) -> Value {
    match value {
        Value::Object(obj) => {
            let filtered = obj
                .iter()
                .filter_map(|(key, val)| {
                    // Build the full path for this key
                    let full_path = if current_path.is_empty() {
                        key.clone()
                    } else {
                        format!("{current_path}.{key}")
                    };

                    // Check if this key or path should be excluded
                    if exclude.contains(key) || exclude.contains(&full_path) {
                        None
                    } else {
                        Some((
                            key.clone(),
                            filter_outputs_recursive(val, exclude, &full_path),
                        ))
                    }
                })
                .collect();
            Value::Object(filtered)
        }
        Value::Array(arr) => {
            let filtered = arr
                .iter()
                .map(|val| filter_outputs_recursive(val, exclude, current_path))
                .collect();
            Value::Array(filtered)
        }
        other => other.clone(),
    }
}

/// Performs a deep comparison of two JSON values.
///
/// This function recursively compares JSON values and provides detailed
/// error messages indicating where mismatches occur.
///
/// # Arguments
///
/// * `expected` - The expected JSON value
/// * `actual` - The actual JSON value
/// * `path` - The current path in the JSON structure (for error messages)
fn compare_json(expected: &Value, actual: &Value, path: &str) -> Result<()> {
    match (expected, actual) {
        (Value::Null, Value::Null) => Ok(()),
        (Value::Bool(e), Value::Bool(a)) => {
            if e == a {
                Ok(())
            } else {
                bail!("boolean mismatch at `{path}`: expected {e}, got {a}")
            }
        }
        (Value::Number(e), Value::Number(a)) => {
            // Compare numbers with floating point tolerance
            let e_f64 = e.as_f64().context("expected number as f64")?;
            let a_f64 = a.as_f64().context("actual number as f64")?;

            if (e_f64 - a_f64).abs() < f64::EPSILON {
                Ok(())
            } else {
                bail!("number mismatch at `{path}`: expected {e_f64}, got {a_f64}")
            }
        }
        (Value::String(e), Value::String(a)) => {
            let e_normalized = normalize_path(e);
            let a_normalized = normalize_path(a);

            if e_normalized == a_normalized {
                Ok(())
            } else {
                bail!("string mismatch at `{path}`: expected \"{e}\", got \"{a}\"")
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            if e.len() != a.len() {
                bail!(
                    "array length mismatch at `{path}`: expected {} elements, got {} elements",
                    e.len(),
                    a.len()
                );
            }

            for (i, (e_val, a_val)) in e.iter().zip(a.iter()).enumerate() {
                let item_path = if path.is_empty() {
                    format!("[{i}]")
                } else {
                    format!("{path}[{i}]")
                };
                compare_json(e_val, a_val, &item_path)?;
            }

            Ok(())
        }
        (Value::Object(e), Value::Object(a)) => {
            // Check for missing keys in actual
            for key in e.keys() {
                if !a.contains_key(key) {
                    let key_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    bail!("missing key in actual output: `{key_path}`");
                }
            }

            // Check for extra keys in actual
            for key in a.keys() {
                if !e.contains_key(key) {
                    let key_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    bail!("unexpected key in actual output: `{key_path}`");
                }
            }

            // Compare values for matching keys
            for (key, e_val) in e.iter() {
                let a_val = &a[key];
                let key_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                compare_json(e_val, a_val, &key_path)?;
            }

            Ok(())
        }
        _ => {
            let expected_type = type_name(expected);
            let actual_type = type_name(actual);
            bail!("type mismatch at `{path}`: expected {expected_type}, got {actual_type}")
        }
    }
}

/// Returns a human-readable type name for a JSON value.
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Normalizes a string value by converting file paths to just their basename.
///
/// This handles differences between WDL engines where some return full absolute
/// paths for `File` and `Directory` types while others return just the basename.
/// If the string represents an existing path on disk, returns just the filename.
/// Otherwise returns the original string.
fn normalize_path(s: &str) -> Cow<'_, str> {
    let path = Path::new(s);
    if path.exists() {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Borrowed(s))
    } else {
        Cow::Borrowed(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_identical_objects() {
        let expected = json!({"a": 1, "b": "test"});
        let actual = json!({"a": 1, "b": "test"});
        assert!(validate_outputs(&expected, &actual, &[]).is_ok());
    }

    #[test]
    fn test_value_mismatch() {
        let expected = json!({"a": 1});
        let actual = json!({"a": 2});
        let result = validate_outputs(&expected, &actual, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("number mismatch"));
    }

    #[test]
    fn test_missing_key() {
        let expected = json!({"a": 1, "b": 2});
        let actual = json!({"a": 1});
        let result = validate_outputs(&expected, &actual, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing key"));
    }

    #[test]
    fn test_extra_key() {
        let expected = json!({"a": 1});
        let actual = json!({"a": 1, "b": 2});
        let result = validate_outputs(&expected, &actual, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unexpected key"));
    }

    #[test]
    fn test_exclude_outputs() {
        let expected = json!({"a": 1, "timestamp": 100});
        let actual = json!({"a": 1, "timestamp": 200});
        assert!(validate_outputs(&expected, &actual, &["timestamp".to_string()]).is_ok());
    }

    #[test]
    fn test_nested_objects() {
        let expected = json!({"outer": {"inner": {"value": 42}}});
        let actual = json!({"outer": {"inner": {"value": 42}}});
        assert!(validate_outputs(&expected, &actual, &[]).is_ok());
    }

    #[test]
    fn test_nested_mismatch() {
        let expected = json!({"outer": {"inner": {"value": 42}}});
        let actual = json!({"outer": {"inner": {"value": 43}}});
        let result = validate_outputs(&expected, &actual, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("outer.inner.value"));
    }

    #[test]
    fn test_array_match() {
        let expected = json!({"items": [1, 2, 3]});
        let actual = json!({"items": [1, 2, 3]});
        assert!(validate_outputs(&expected, &actual, &[]).is_ok());
    }

    #[test]
    fn test_array_length_mismatch() {
        let expected = json!({"items": [1, 2, 3]});
        let actual = json!({"items": [1, 2]});
        let result = validate_outputs(&expected, &actual, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("array length mismatch"));
    }

    #[test]
    fn test_array_element_mismatch() {
        let expected = json!({"items": [1, 2, 3]});
        let actual = json!({"items": [1, 5, 3]});
        let result = validate_outputs(&expected, &actual, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("items[1]"));
    }

    #[test]
    fn test_type_mismatch() {
        let expected = json!({"value": 42});
        let actual = json!({"value": "42"});
        let result = validate_outputs(&expected, &actual, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("type mismatch"));
    }

    #[test]
    fn test_exclude_nested_key() {
        let expected = json!({"a": 1, "nested": {"timestamp": 100, "value": 42}});
        let actual = json!({"a": 1, "nested": {"timestamp": 200, "value": 42}});
        assert!(validate_outputs(&expected, &actual, &["timestamp".to_string()]).is_ok());
    }

    #[test]
    fn test_exclude_nested_path() {
        let expected = json!({"a": 1, "nested": {"timestamp": 100, "value": 42}});
        let actual = json!({"a": 1, "nested": {"timestamp": 200, "value": 42}});
        assert!(validate_outputs(&expected, &actual, &["nested.timestamp".to_string()]).is_ok());
    }

    #[test]
    fn test_exclude_nested_path_preserves_other_fields() {
        let expected = json!({"a": 1, "nested": {"timestamp": 100, "value": 42}});
        let actual = json!({"a": 1, "nested": {"timestamp": 200, "value": 99}});
        let result = validate_outputs(&expected, &actual, &["nested.timestamp".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nested.value"));
    }
}
