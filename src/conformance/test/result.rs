//! Results of a conformance test.

use std::fmt;

use crate::conformance::test::ReturnCode;
use crate::conformance::Capability;

/// The result of running a conformance test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestResult {
    /// The test passed.
    Passed,
    /// The test failed.
    Failed(FailureReason),
    /// The test was skipped.
    Skipped(SkipReason),
}

/// The reason a test failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureReason {
    /// The return code did not match the expected value.
    ReturnCodeMismatch {
        /// The expected return code(s).
        expected: ReturnCode,
        /// The actual return code.
        actual: i32,
    },
    /// The output did not match the expected value.
    OutputMismatch {
        /// Details about the mismatch.
        details: String,
    },
    /// The command execution failed with an error.
    ExecutionError(String),
    /// The test was expected to fail but succeeded.
    UnexpectedSuccess,
    /// No output was produced by the command.
    NoOutput,
    /// The output selector failed.
    SelectorError {
        /// The selector that failed.
        selector: String,
        /// Details about the error.
        details: String,
    },
}

/// The reason a test was skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// The test was explicitly ignored.
    Ignored,
    /// The test requires capabilities that were not provided.
    MissingCapabilities(Vec<Capability>),
}

impl TestResult {
    /// Returns `true` if the test passed.
    pub fn is_passed(&self) -> bool {
        matches!(self, TestResult::Passed)
    }

    /// Returns `true` if the test failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, TestResult::Failed(_))
    }

    /// Returns `true` if the test was skipped.
    pub fn is_skipped(&self) -> bool {
        matches!(self, TestResult::Skipped(_))
    }
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FailureReason::ReturnCodeMismatch { expected, actual } => {
                write!(
                    f,
                    "return code mismatch: expected {:?}, got {}",
                    expected, actual
                )
            }
            FailureReason::OutputMismatch { details } => {
                write!(f, "output mismatch: {}", details)
            }
            FailureReason::ExecutionError(e) => {
                write!(f, "execution error: {}", e)
            }
            FailureReason::UnexpectedSuccess => {
                write!(f, "test marked with `fail: true` but succeeded")
            }
            FailureReason::NoOutput => {
                write!(f, "no output produced—the command may have failed")
            }
            FailureReason::SelectorError { selector, details } => {
                write!(f, "selector error for `{}`: {}", selector, details)
            }
        }
    }
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkipReason::Ignored => write!(f, "test marked with `ignore: true`"),
            SkipReason::MissingCapabilities(caps) => {
                let caps_str = caps
                    .iter()
                    .map(|c| format!("`{}`", c))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "missing required capabilities: {}", caps_str)
            }
        }
    }
}
