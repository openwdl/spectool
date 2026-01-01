//! Conformance test parsing from within `SPEC.md`.

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use bon::Builder;
use regex::Captures;
use regex::Regex;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::wdl;

mod config;
pub mod result;
pub mod runner;
pub mod validation;

pub use config::Capability;
pub use config::Config;
pub use config::ReturnCode;
pub use config::Tag;
pub use result::FailureReason;
pub use result::SkipReason;
pub use result::TestResult;
pub use runner::Runner;
pub use wdl::Target;

/// The regex for a WDL conformance test within the specification.
static CONFORMANCE_TEST_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    const PATTERN: &str = concat!(
        "(?is)", // Turn on `i` and `s` options.
        r"<details>\s*",
        r"<summary>\s*",
        r"Example: (.+?)\s*```wdl(.+?)```\s*",
        r"</summary>\s*",
        r"(?:<p>\s*",
        r"(?:Example input:\s*```json(.*?)```)?\s*",
        r"(?:Example output:\s*```json(.*?)```)?\s*",
        r"(?:Test config:\s*```json(.*?)```)?\s*",
        r"</p>\s*",
        r")?",
        r"</details>"
    );

    Regex::new(PATTERN).unwrap()
});

/// A conformance test.
#[derive(Builder, Debug)]
#[builder(builder_type = Builder)]
pub struct Test {
    /// The path to the test, if has been written.
    path: Option<PathBuf>,

    /// The file name of the test.
    file_name: String,

    /// The source.
    src: String,

    /// The input.
    input: Option<Value>,

    /// The output.
    output: Option<Value>,

    /// The configuration.
    config: Config,

    /// The inferred or validated target workflow/task.
    inferred_target: Option<wdl::Target>,
}

impl Test {
    /// The path to the test.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file name of the test.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The source of the test.
    pub fn src(&self) -> &str {
        &self.src
    }

    /// The input of the test.
    pub fn input(&self) -> Option<&Value> {
        self.input.as_ref()
    }

    /// The output of the test.
    pub fn output(&self) -> Option<&Value> {
        self.output.as_ref()
    }

    /// The configuration of the test.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Sets the path for the test.
    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    /// Gets the target workflow or task.
    ///
    /// Returns the inferred target if it has been set, otherwise `None`.
    pub fn target(&self) -> Option<&wdl::Target> {
        self.inferred_target.as_ref()
    }

    /// Infers and validates the target workflow/task name according to
    /// `SPEC.md` rules.
    ///
    /// This method must be called after test construction to determine what to
    /// execute.
    pub fn infer_and_validate_target(&mut self) -> Result<()> {
        let decls = wdl::parse_wdl_declarations(&self.src).context("parsing WDL declarations")?;

        // Check if there's a single unambiguous target.
        let single_target = decls.single_target();

        // Check if target can be inferred from the input JSON.
        let input_inferred_target = self.infer_target_from_input(&decls)?;

        // Get the explicit target from config.
        let config_target = self.config.target();

        // Apply validation rules from SPEC.md
        match (single_target, input_inferred_target.as_ref(), config_target) {
            // If target can be inferred but `config.target` is provided, error.
            (Some(_), _, Some(_)) => {
                bail!(
                    "target should not be specified in config, as it can be inferred from the WDL directly (test: `{}`)",
                    self.file_name
                );
            }
            (_, Some(_), Some(_)) => {
                bail!(
                    "target should not be specified in config, as it can be inferred from the input JSON directly (test: `{}`)",
                    self.file_name
                );
            }

            // If single target exists, use it.
            (Some(target), None, None) => {
                self.inferred_target = Some(target);
                Ok(())
            }

            // If can infer from input JSON, use that.
            (None, Some(target), None) => {
                self.inferred_target = Some(target.clone());
                Ok(())
            }

            // If both single target and input infer the same target, use it.
            (Some(single), Some(input), None) if single == *input => {
                self.inferred_target = Some(single);
                Ok(())
            }

            // If single target and input disagree, error.
            (Some(single), Some(input), None) => {
                bail!(
                    "conflicting target inference: WDL structure suggests `{:?}` but input suggests `{:?}` (test: `{}`)",
                    single,
                    input,
                    self.file_name
                );
            }

            // Multiple tasks, no input, no config target, error.
            (None, None, None) if !decls.tasks().is_empty() => {
                bail!(
                    "target required in config: cannot infer which task to run (test: `{}`)",
                    self.file_name,
                );
            }

            // Multiple tasks, no input, config target provided, ok.
            (None, None, Some(target)) if !decls.tasks().is_empty() => {
                // Validate that the target actually exists in the tasks
                if !decls.tasks().contains(&target.to_string()) {
                    bail!(
                        "target `{}` not found in tasks (test: `{}`)",
                        target,
                        self.file_name
                    );
                }
                // Since we validated it's in tasks list, it's a Task
                self.inferred_target = Some(wdl::Target::Task(target.to_string()));
                Ok(())
            }

            // No workflow, no tasks, error.
            (None, None, _) if decls.tasks().is_empty() && decls.workflow().is_none() => {
                bail!(
                    "no workflow or task found in WDL source (test: `{}`)",
                    self.file_name
                );
            }

            // Should not reach here.
            _ => {
                bail!(
                    "unexpected target inference state (test: `{}`)",
                    self.file_name
                );
            }
        }
    }

    /// Attempts to infer the target from input JSON parameter prefixes.
    ///
    /// Returns `Some(target)` if all input parameters share a common prefix.
    /// Returns `None` if there are no inputs or no common prefix can be determined.
    fn infer_target_from_input(&self, decls: &wdl::WdlDeclarations) -> Result<Option<wdl::Target>> {
        let input = match &self.input {
            Some(input) => input,
            None => return Ok(None),
        };

        let obj = match input.as_object() {
            Some(obj) => obj,
            None => return Ok(None),
        };

        if obj.is_empty() {
            return Ok(None);
        }

        // Extract unique prefixes from input keys (e.g., "my_task.input1" → "my_task")
        let prefixes = obj
            .keys()
            .filter_map(|key| key.split('.').next().map(|s| s.to_string()))
            .collect::<HashSet<String>>();

        // Must have exactly one unique prefix
        if prefixes.len() == 1 {
            let prefix = prefixes.into_iter().next().unwrap();

            // Check if prefix matches workflow or task
            if matches!(decls.workflow(), Some(wf) if wf == prefix) {
                Ok(Some(wdl::Target::Workflow(prefix)))
            } else if decls.tasks().contains(&prefix) {
                Ok(Some(wdl::Target::Task(prefix)))
            } else {
                bail!(
                    "input prefix `{}` does not match any workflow or task in WDL (test: `{}`)",
                    prefix,
                    self.file_name
                );
            }
        } else if prefixes.len() > 1 {
            bail!("ambiguous input prefixes (test: `{}`)", self.file_name);
        } else {
            Ok(None)
        }
    }
}

/// A set of conformance tests.
pub struct Tests(Vec<Test>);

impl Tests {
    /// Turns a markdown specification into a set of conformance tests.
    pub fn compile<S: AsRef<str>>(contents: S) -> Result<Self> {
        let contents = contents.as_ref();

        let tests = CONFORMANCE_TEST_REGEX
            .captures_iter(contents)
            .map(build_conformance_test)
            .collect::<Result<Vec<Test>, _>>()?;

        Ok(Self(tests))
    }

    /// Returns a reference to each conformance test.
    pub fn tests(&self) -> impl Iterator<Item = &Test> {
        self.0.iter()
    }

    /// Returns a mutable reference to each conformance test.
    pub fn tests_mut(&mut self) -> impl Iterator<Item = &mut Test> {
        self.0.iter_mut()
    }

    /// Consumes `self` and returns the conformance tests.
    pub fn into_tests(self) -> impl Iterator<Item = Test> {
        self.0.into_iter()
    }
}

/// Builds a conformance test from a set of captures.
fn build_conformance_test(captures: Captures<'_>) -> Result<Test> {
    let file_name = required_string(&captures, 1, "filename")?;
    let src = required_string(&captures, 2, "source")?;
    let input = optional_json_group(&captures, 3);
    let output = optional_json_group(&captures, 4);
    let config = optional_group::<Config>(&captures, 5)?.unwrap_or_default();

    Ok(Test::builder()
        .file_name(file_name)
        .src(src)
        .maybe_input(input)
        .maybe_output(output)
        .config(config)
        .build())
}

/// Parses a _required_ group within a test.
fn required_string(captures: &Captures<'_>, index: usize, name: &str) -> Result<String> {
    captures
        .get(index)
        .ok_or_else(|| {
            anyhow!(
                "unable to parse {} from test:\n\n{}",
                name,
                captures.get(0).unwrap().as_str()
            )
        })
        .map(|v| v.as_str().to_owned())
}

/// Parses an _optional_ group within a test.
fn optional_json_group(captures: &Captures<'_>, index: usize) -> Option<Value> {
    captures.get(index).and_then(|v| v.as_str().parse().ok())
}

/// Parses an _optional_ group within a test.
fn optional_group<D>(captures: &Captures<'_>, index: usize) -> Result<Option<D>>
where
    D: DeserializeOwned,
{
    captures
        .get(index)
        .map(|m| {
            serde_json::from_str::<D>(m.as_str()).with_context(|| {
                format!(
                    "parsing configuration:\n\n{}",
                    captures.get(0).unwrap().as_str()
                )
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_matches() {
        let example = r#"
        <details>
            <summary>
                Example: hello.wdl

                ```wdl
                version 1.2

                task hello_task {
                    input {
                    File infile
                    String pattern
                    }

                    command <<<
                    grep -E '~{pattern}' '~{infile}'
                    >>>

                    requirements {
                    container: "ubuntu:latest"
                    }

                    output {
                    Array[String] matches = read_lines(stdout())
                    }
                }

                workflow hello {
                    input {
                    File infile
                    String pattern
                    }

                    call hello_task {
                    infile, pattern
                    }

                    output {
                    Array[String] matches = hello_task.matches
                    }
                }
                ```
            </summary>
            <p>
            Example input:

            ```json
            {
                "hello.infile": "greetings.txt",
                "hello.pattern": "hello.*"
            }
            ```

            Example output:

            ```json
            {
                "hello.matches": ["hello world", "hello nurse"]
            }
            ```
            </p>
        </details>"#;

        let captures = CONFORMANCE_TEST_REGEX
            .find_iter(example)
            .collect::<Vec<_>>();
        assert_eq!(captures.len(), 1);
    }
}
