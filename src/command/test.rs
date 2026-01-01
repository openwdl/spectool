//! A subcommand to run the conformance tests.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use strum::IntoEnumIterator;

use crate::Repository;
use crate::badge::Badge;
use crate::conformance::Capability;
use crate::conformance::FailureReason;
use crate::conformance::ReturnCode;
use crate::conformance::SkipReason;
use crate::conformance::Test;
use crate::conformance::TestResult;
use crate::conformance::test::Runner;
use crate::conformance::test::validation::validate_outputs;
use crate::shell::substitute;

/// The file name of the specification.
const SPEC_FILE_NAME: &str = "SPEC.md";

/// Performs conformance tests on the WDL specification.
#[derive(Parser, Debug)]
pub struct Args {
    /// The branch to check out.
    #[arg(short, long, default_value = "wdl-1.2")]
    branch: String,

    /// The git repository URL to clone.
    #[arg(long, default_value = "https://github.com/openwdl/wdl.git")]
    repository_url: String,

    /// A directory that contains the conformance tests.
    #[arg(short, long)]
    conformance_test_dir: Option<PathBuf>,

    /// Whether to force the writing of the conformance tests directory.
    #[arg(short, long, default_value_t = false)]
    force: bool,

    /// A directory that contains the specification repository.
    #[arg(short, long)]
    specification_dir: Option<PathBuf>,

    /// Runtime capabilities available for tests (comma-separated).
    ///
    /// Tests requiring capabilities not in this list will be skipped.
    #[arg(long, value_delimiter = ',', conflicts_with = "all_capabilities")]
    capabilities: Vec<Capability>,

    /// Enable all runtime capabilities.
    #[arg(long, conflicts_with = "capabilities")]
    all_capabilities: bool,

    /// Arguments to append when running a workflow.
    ///
    /// Use `~{target}` for the workflow name.
    #[arg(long, default_value = "")]
    workflow_target_args: String,

    /// Arguments to append when running a task.
    ///
    /// Use `~{target}` for the task name.
    #[arg(long, default_value = "")]
    task_target_args: String,

    /// Redirect stdout to the outputs file.
    ///
    /// If enabled, appends `> ~{output}` at the end of the command.
    #[arg(long, default_value_t = false)]
    redirect_stdout: bool,

    /// Only run tests matching these patterns (comma-separated).
    ///
    /// Patterns are matched as substrings of test names.
    /// Mutually exclusive with `--exclude`.
    #[arg(long, value_delimiter = ',', conflicts_with = "exclude")]
    include: Vec<String>,

    /// Skip tests matching these patterns (comma-separated).
    ///
    /// Patterns are matched as substrings of test names.
    /// Mutually exclusive with `--include`.
    #[arg(long, value_delimiter = ',', conflicts_with = "include")]
    exclude: Vec<String>,

    /// A `jq` selector to apply to `outputs.json` before validation.
    ///
    /// This allows transforming the output JSON before comparing against expected output.
    /// For example, `--output-selector '.outputs'` will extract the `outputs` field from the output.
    ///
    /// Uses `jq` syntax (e.g., `'.outputs'`, `'.result.data[0]'`, etc.).
    #[arg(long)]
    output_selector: Option<String>,

    /// WDL version to inject into test files.
    ///
    /// Replaces the `version` statement in each test file before writing to disk.
    /// For example, `--inject-wdl-version development` will replace `version 1.2`
    /// with `version development`.
    ///
    /// This is useful when testing against engines that require specific version strings.
    #[arg(long, value_name = "VERSION")]
    inject_wdl_version: Option<String>,

    /// Label for JSON badge output to stdout.
    ///
    /// The badge is output in Shields.io endpoint format with test results.
    /// For example, `--label "Cromwell WDL 1.2"` outputs:
    /// `{"schemaVersion": 1, "label": "Sprocket WDL 1.2", "message": "157/169 passed", "color": "yellow"}`
    #[arg(long, default_value = "Spectool")]
    label: String,

    /// The command to call for each execution.
    ///
    #[arg(help = r#"The command to call for each execution.

The following substitutions are supported:

  - `~{path}` is the path to the file.
  - `~{input}` is the path to the inputs.json file.
  - `~{output}` is the path to the outputs.json file."#)]
    command: String,
}

/// The main method.
pub fn main(mut args: Args) -> Result<()> {
    //======================//
    // Handle capabilities //
    //======================//

    if args.all_capabilities {
        args.capabilities = Capability::iter().collect();
    }

    //=======================================//
    // Checkout the specification repository //
    //=======================================//

    let (_, path) = Repository::builder()
        .branch(args.branch)
        .url(args.repository_url)
        .maybe_local_dir(args.specification_dir)
        .build()
        .checkout()?;

    //=================================//
    // Read the specification contents //
    //=================================//

    let spec = path.join(SPEC_FILE_NAME);

    if !spec.exists() {
        bail!(
            "the specification does not exist at `{}` in the git repository",
            SPEC_FILE_NAME
        );
    }

    let contents = std::fs::read_to_string(spec)?;

    //===============================//
    // Compile the conformance tests //
    //===============================//

    let root_dir = args
        .conformance_test_dir
        .map(|path| std::path::absolute(path).expect("path to be made absolute"))
        .unwrap_or_else(|| tempfile::tempdir().expect("tempdir to create").into_path());

    let runner = Runner::compile(
        root_dir,
        contents,
        args.force,
        args.inject_wdl_version.clone(),
    )?;

    //===================================//
    // Set up the test working directory //
    //===================================//

    // SAFETY: this should create on all platforms we care about.
    let workdir = tempfile::tempdir().expect("tempdir to create").into_path();

    //===============//
    // Run the tests //
    //===============//

    let mut results = Vec::new();
    let mut total_elapsed = std::time::Duration::ZERO;

    for test in runner.tests() {
        // (1) Check if test should be filtered by include/exclude
        let test_name = test.file_name().trim_end_matches(".wdl");
        if !args.include.is_empty()
            && !args
                .include
                .iter()
                .any(|pattern| test_name.contains(pattern.as_str()))
        {
            continue;
        }
        if !args.exclude.is_empty()
            && args
                .exclude
                .iter()
                .any(|pattern| test_name.contains(pattern.as_str()))
        {
            continue;
        }

        // (2) Check if test should be ignored
        if test.config().ignore() {
            print_result(
                test.file_name(),
                "SKIP",
                Some("test marked with `ignore: true`"),
                None,
            );
            results.push((
                test.file_name().to_string(),
                TestResult::Skipped(SkipReason::Ignored),
            ));
            continue;
        }

        // (2) Check if test has required capabilities
        let missing_capabilities: Vec<Capability> = test
            .config()
            .capabilities()
            .iter()
            .filter(|cap| !args.capabilities.contains(cap))
            .cloned()
            .collect();

        if !missing_capabilities.is_empty() {
            let reason = SkipReason::MissingCapabilities(missing_capabilities);
            print_result(test.file_name(), "SKIP", Some(&reason.to_string()), None);
            results.push((test.file_name().to_string(), TestResult::Skipped(reason)));
            continue;
        }

        // (3) Recreate the working directory to ensure it's empty
        // SAFETY: we expect to be able to remove and recreate the directory on all
        // platforms we care about within this subcommand.
        std::fs::remove_dir_all(&workdir).unwrap();
        std::fs::create_dir_all(&workdir).unwrap();

        // (4) Copy data directory to the working directory
        let source_data_dir = runner.root_dir().join("data");
        let dest_data_dir = &workdir;
        if source_data_dir.exists() {
            let mut options = fs_extra::dir::CopyOptions::new();
            options.overwrite = true;
            options.copy_inside = true;
            // SAFETY: we expect to be able to copy the `data` directory on all
            // platforms we care about within this subcommand.
            fs_extra::dir::copy(&source_data_dir, dest_data_dir, &options).unwrap();
        }

        // (5) Create the inputs file
        let input_file = create_input_json(test, &workdir).unwrap();

        // (5) Substitute the command
        let target = test.target().expect("target should be inferred");
        let output_file = workdir.join("outputs.json");
        let command = substitute()
            .command(args.command.clone())
            .path(test.path().unwrap().to_path_buf())
            .input(input_file)
            .output(output_file)
            .target(target.clone())
            .workflow_target_args(args.workflow_target_args.clone())
            .task_target_args(args.task_target_args.clone())
            .call();

        tracing::debug!("executing command `{}`", command);

        // (6) Execute the test and evaluate the result
        let start_time = std::time::Instant::now();
        let result = execute_and_evaluate_test(
            test,
            &command,
            runner.root_dir(),
            &workdir,
            args.redirect_stdout,
            args.output_selector.as_deref(),
        );
        let elapsed = start_time.elapsed();
        total_elapsed += elapsed;

        // (8) Print result and store it
        match &result {
            TestResult::Passed => print_result(test.file_name(), "PASS", None, Some(elapsed)),
            TestResult::Failed(reason) => {
                print_result(
                    test.file_name(),
                    "FAIL",
                    Some(&reason.to_string()),
                    Some(elapsed),
                );
            }
            TestResult::Skipped(reason) => {
                print_result(
                    test.file_name(),
                    "SKIP",
                    Some(&reason.to_string()),
                    Some(elapsed),
                );
            }
        }

        results.push((test.file_name().to_string(), result));
    }

    //===================//
    // Print summary     //
    //===================//

    eprintln!("\n{}", "=".repeat(60));
    eprintln!("Test Summary");
    eprintln!("{}", "=".repeat(60));
    eprintln!();

    let passed = results.iter().filter(|(_, r)| r.is_passed()).count();
    let failed = results.iter().filter(|(_, r)| r.is_failed()).count();
    let skipped = results.iter().filter(|(_, r)| r.is_skipped()).count();

    eprintln!("Passed:  {}", passed);
    eprintln!("Failed:  {}", failed);
    eprintln!("Skipped: {}", skipped);
    eprintln!("Total:   {}", passed + failed);
    eprintln!();
    eprintln!("Total time:   {:.2}s", total_elapsed.as_secs_f64());

    let executed = passed + failed;
    if executed > 0 {
        let avg_time = total_elapsed.as_secs_f64() / executed as f64;
        eprintln!("Average time: {:.2}s per test", avg_time);
    }

    //=======================//
    // Output JSON to stdout //
    //=======================//

    let badge_passed = results.iter().filter(|(_, r)| r.is_passed()).count();
    let badge_failed = results.iter().filter(|(_, r)| r.is_failed()).count();
    let badge_total = badge_passed + badge_failed;

    Badge::from_results(args.label, badge_passed, badge_total).output();

    if failed > 0 {
        bail!("{} test(s) failed", failed);
    }

    Ok(())
}

/// Creates an `input.json` file.
fn create_input_json(test: &Test, work_dir: &Path) -> Result<PathBuf> {
    let input = match test.input() {
        Some(value) => serde_json::to_string_pretty(value).context("serializing input file")?,
        None => Default::default(),
    };

    let input_file_path = work_dir.join("inputs.json");
    std::fs::write(&input_file_path, input).context("writing `inputs.json` file")?;

    Ok(input_file_path)
}

/// Executes a test and evaluates the result.
fn execute_and_evaluate_test(
    test: &Test,
    command: &str,
    root_dir: &Path,
    workdir: &Path,
    redirect_stdout: bool,
    output_selector: Option<&str>,
) -> TestResult {
    // Execute the command
    let output = match Command::new("bash")
        .args(["-c", command])
        .current_dir(root_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            return TestResult::Failed(FailureReason::ExecutionError(e.to_string()));
        }
    };

    let exit_code = output.status.code().unwrap_or(-1);

    tracing::trace!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    tracing::trace!("stderr: {}", String::from_utf8_lossy(&output.stderr));

    // Write stdout to `outputs.json` if `redirect_stdout` is enabled
    if redirect_stdout {
        let outputs_path = workdir.join("outputs.json");
        if let Err(e) = std::fs::write(&outputs_path, &output.stdout) {
            return TestResult::Failed(FailureReason::ExecutionError(format!(
                "failed to write stdout to `outputs.json`: {}",
                e
            )));
        }
    }

    // Determine if test should have failed
    let expected_to_fail = test.config().fail();

    // If test is expected to fail, check if command failed (non-zero exit)
    if expected_to_fail {
        if exit_code == 0 {
            return TestResult::Failed(FailureReason::UnexpectedSuccess);
        } else {
            return TestResult::Passed;
        }
    }

    // Check return code
    let return_code_matches = match test.config().return_code() {
        ReturnCode::Any => true,
        ReturnCode::Single(expected) => exit_code == *expected,
        ReturnCode::Multiple(expected) => expected.contains(&exit_code),
    };

    // If return code doesn't match, test failed
    if !return_code_matches {
        return TestResult::Failed(FailureReason::ReturnCodeMismatch {
            expected: test.config().return_code().clone(),
            actual: exit_code,
        });
    }

    // If we have expected output, validate it
    if let Some(expected_output) = test.output() {
        let outputs_path = workdir.join("outputs.json");

        let actual_output = match std::fs::read_to_string(&outputs_path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return TestResult::Failed(FailureReason::NoOutput);
            }
            Err(e) => {
                return TestResult::Failed(FailureReason::OutputMismatch {
                    details: format!("failed to read `outputs.json`: {}", e),
                });
            }
        };

        // Check if `outputs.json` is empty
        if actual_output.trim().is_empty() {
            return TestResult::Failed(FailureReason::NoOutput);
        }

        let actual_output: serde_json::Value = match serde_json::from_str(&actual_output) {
            Ok(value) => value,
            Err(e) => {
                return TestResult::Failed(FailureReason::OutputMismatch {
                    details: format!("failed to parse `outputs.json`: {}", e),
                });
            }
        };

        // Apply output selector if provided
        let actual_output = if let Some(selector) = output_selector {
            match apply_selector(selector, &actual_output) {
                Ok(transformed) => transformed,
                Err(failure_reason) => return TestResult::Failed(failure_reason),
            }
        } else {
            actual_output
        };

        if let Err(e) = validate_outputs(
            expected_output,
            &actual_output,
            test.config().exclude_outputs(),
        ) {
            return TestResult::Failed(FailureReason::OutputMismatch {
                details: e.to_string(),
            });
        }
    }

    TestResult::Passed
}

/// Prints a test result in the format: <test_name>...RESULT [time]
fn print_result(
    test_name: &str,
    status: &str,
    details: Option<&str>,
    elapsed: Option<std::time::Duration>,
) {
    const TOTAL_WIDTH: usize = 50;

    let dots_len = TOTAL_WIDTH.saturating_sub(test_name.len());
    let dots = ".".repeat(dots_len);

    let (color_code, reset_code) = match status {
        "PASS" => ("\x1b[32m", "\x1b[0m"), // Green
        "FAIL" => ("\x1b[31m", "\x1b[0m"), // Red
        "SKIP" => ("\x1b[33m", "\x1b[0m"), // Yellow
        _ => ("", ""),
    };

    let time_str = elapsed
        .map(|d| format!(" [{:.2}s]", d.as_secs_f64()))
        .unwrap_or_default();

    if let Some(details_str) = details {
        eprintln!(
            "{}{}{}{}{}{} ({})",
            test_name, dots, color_code, status, reset_code, time_str, details_str
        );
    } else {
        eprintln!(
            "{}{}{}{}{}{}",
            test_name, dots, color_code, status, reset_code, time_str
        );
    }
}

/// Applies a `jq` selector to a JSON value.
fn apply_selector(
    selector: &str,
    input: &serde_json::Value,
) -> Result<serde_json::Value, FailureReason> {
    use jaq_core::load::{Arena, File, Loader};
    use jaq_core::{Compiler, Ctx, Vars, data, unwrap_valr};
    use jaq_json::Val;

    let program = File {
        code: selector,
        path: (),
    };
    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = Arena::default();

    // Parse the selector
    let modules = loader.load(&arena, program).map_err(|errs| {
        let error_msg = errs
            .into_iter()
            .map(|(file, err)| format!("{}: {:?}", file.code, err))
            .collect::<Vec<_>>()
            .join("; ");
        FailureReason::SelectorError {
            selector: selector.to_string(),
            details: error_msg,
        }
    })?;

    // Compile the selector
    let filter = Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|errs| {
            let error_msg = errs
                .into_iter()
                .map(|(file, err)| {
                    let err_str = err
                        .into_iter()
                        .map(|(name, _)| name)
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}: undefined: {}", file.code, err_str)
                })
                .collect::<Vec<_>>()
                .join("; ");
            FailureReason::SelectorError {
                selector: selector.to_string(),
                details: error_msg,
            }
        })?;

    // Convert `serde_json::Value` to `jaq` `Val` using JSON string roundtrip
    let json_str = input.to_string();
    let jaq_input = jaq_json::read::parse_single(json_str.as_bytes()).map_err(|e| {
        FailureReason::SelectorError {
            selector: selector.to_string(),
            details: format!("failed to parse input as JSON: {}", e),
        }
    })?;

    // Execute the selector
    let ctx = Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]));
    let mut outputs = filter.id.run((ctx, jaq_input)).map(unwrap_valr);

    // Expect exactly one output
    let first_output = outputs.next();
    let second_output = outputs.next();

    match (first_output, second_output) {
        (None, _) => Err(FailureReason::SelectorError {
            selector: selector.to_string(),
            details: "selector produced no output".to_string(),
        }),
        (Some(Err(e)), _) => Err(FailureReason::SelectorError {
            selector: selector.to_string(),
            details: format!("selector execution failed: {}", e),
        }),
        (Some(Ok(_)), Some(_)) => Err(FailureReason::SelectorError {
            selector: selector.to_string(),
            details: "selector produced multiple outputs (expected exactly one)".to_string(),
        }),
        (Some(Ok(val)), None) => {
            let json_str = val.to_string();
            serde_json::from_str(&json_str).map_err(|e| FailureReason::SelectorError {
                selector: selector.to_string(),
                details: format!("failed to convert result to JSON: {}", e),
            })
        }
    }
}
