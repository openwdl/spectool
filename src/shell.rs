//! Shell faculties for substitutions.

use std::path::PathBuf;

use bon::builder;

use crate::conformance::Target;

/// Builds the command with substitutions and target-specific arguments.
///
/// Substitutions:
///
/// - `~{path}` → path to the WDL file
/// - `~{input}` → path to the inputs.json file
/// - `~{output}` → path to the outputs.json file
/// - `~{target}` → workflow or task name
///
/// The appropriate target args template is selected based on the target type
/// and appended to the command after substitutions.
#[builder]
pub fn substitute(
    mut command: String,
    path: PathBuf,
    input: PathBuf,
    output: PathBuf,
    target: Target,
    workflow_target_args: String,
    task_target_args: String,
) -> String {
    // Select the appropriate target args template and substitute target name
    let target_args = match &target {
        Target::Workflow(_) => workflow_target_args,
        Target::Task(_) => task_target_args,
    };

    // Append target args to command
    command.push(' ');
    command.push_str(&target_args);

    command = command.replace("~{path}", &path.display().to_string());
    command = command.replace("~{input}", &input.display().to_string());
    command = command.replace("~{output}", &output.display().to_string());
    command = command.replace("~{target}", target.name());
    command.trim().to_string()
}
