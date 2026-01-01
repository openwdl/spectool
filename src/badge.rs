//! Badge generation for test results.

use serde::Serialize;

/// A shields.io endpoint badge.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Badge {
    /// Schema version (always `1`).
    schema_version: u8,
    /// Badge label (left side).
    label: String,
    /// Badge message (right side).
    message: String,
    /// Badge color.
    color: String,
}

impl Badge {
    /// Creates a new badge from test results.
    ///
    /// The message is formatted as `"{passed}/{total} passed"`.
    ///
    /// The color is determined by pass rate:
    ///
    /// - `"brightgreen"` for 100% pass rate
    /// - `"yellow"` for 50-99% pass rate
    /// - `"red"` for < 50% pass rate
    /// - `"lightgrey"` for 0 total tests
    pub fn from_results(label: String, passed: usize, total: usize) -> Self {
        let color = determine_color(passed, total);
        let message = format!("{}/{} passed", passed, total);

        Self {
            schema_version: 1,
            label,
            message,
            color,
        }
    }

    /// Outputs the badge as JSON to stdout.
    pub fn output(&self) {
        let json = serde_json::to_string_pretty(self).expect("badge serialization to succeed");
        println!("{}", json);
    }
}

/// Determines badge color based on pass rate.
fn determine_color(passed: usize, total: usize) -> String {
    if total == 0 {
        return String::from("lightgrey");
    }

    let pass_rate = (passed as f64) / (total as f64);

    if pass_rate >= 1.0 {
        String::from("brightgreen")
    } else if pass_rate >= 0.5 {
        String::from("yellow")
    } else {
        String::from("red")
    }
}
