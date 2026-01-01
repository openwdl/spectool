//! Simple WDL inference faculties.

use std::sync::LazyLock;

use anyhow::Result;
use regex::Regex;

/// A target to execute in a WDL file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// A task target.
    Task(String),
    /// A workflow target.
    Workflow(String),
}

impl Target {
    /// Gets the name of the target.
    pub fn name(&self) -> &str {
        match self {
            Target::Task(name) => name,
            Target::Workflow(name) => name,
        }
    }
}

/// Regex to match workflow declarations in WDL.
static WORKFLOW_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*workflow\s+(\w+)\s*\{").unwrap());

/// Regex to match task declarations in WDL.
static TASK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\s*task\s+(\w+)\s*\{").unwrap());

/// The declarations found in a WDL file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WdlDeclarations {
    /// The workflow name, if one exists.
    workflow: Option<String>,
    /// The task names.
    tasks: Vec<String>,
}

impl WdlDeclarations {
    /// Gets the workflow name.
    pub fn workflow(&self) -> Option<&str> {
        self.workflow.as_deref()
    }

    /// Gets the task names.
    pub fn tasks(&self) -> &[String] {
        &self.tasks
    }

    /// Returns the single workflow or task target, if there is exactly one target to run.
    ///
    /// Returns `Some` if:
    /// - There is a workflow (regardless of tasks)
    /// - There is no workflow and exactly one task
    ///
    /// Returns `None` if:
    /// - There is no workflow and zero or multiple tasks
    pub fn single_target(&self) -> Option<Target> {
        match (&self.workflow, self.tasks.as_slice()) {
            (Some(wf), _) => Some(Target::Workflow(wf.clone())), // Workflow always takes precedence
            (None, [task]) => Some(Target::Task(task.clone())),
            _ => None,
        }
    }
}

/// Parses WDL source code to extract workflow and task declarations.
///
/// This is a minimal regex-based parser that only extracts declaration names,
/// not a full WDL parser.
pub fn parse_wdl_declarations(source: &str) -> Result<WdlDeclarations> {
    // Extract workflow name (should be at most one)
    let workflow = WORKFLOW_REGEX
        .captures(source)
        .map(|cap| cap[1].to_string());

    // Extract all task names
    let tasks = TASK_REGEX
        .captures_iter(source)
        .map(|cap| cap[1].to_string())
        .collect();

    Ok(WdlDeclarations { workflow, tasks })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_workflow() {
        let wdl = r#"
            version 1.2
            workflow hello {
                input {
                    String name
                }
            }
        "#;

        let decls = parse_wdl_declarations(wdl).unwrap();
        assert_eq!(decls.workflow(), Some("hello"));
        assert_eq!(decls.tasks(), &[] as &[String]);
        assert_eq!(
            decls.single_target(),
            Some(Target::Workflow("hello".to_string()))
        );
    }

    #[test]
    fn single_task() {
        let wdl = r#"
            version 1.2
            task my_task {
                command {
                    echo "hello"
                }
            }
        "#;

        let decls = parse_wdl_declarations(wdl).unwrap();
        assert_eq!(decls.workflow(), None);
        assert_eq!(decls.tasks(), &["my_task"]);
        assert_eq!(
            decls.single_target(),
            Some(Target::Task("my_task".to_string()))
        );
    }

    #[test]
    fn workflow_and_tasks() {
        let wdl = r#"
            version 1.2

            task task1 {
                command { echo "1" }
            }

            task task2 {
                command { echo "2" }
            }

            workflow my_workflow {
                call task1
                call task2
            }
        "#;

        let decls = parse_wdl_declarations(wdl).unwrap();
        assert_eq!(decls.workflow(), Some("my_workflow"));
        assert_eq!(decls.tasks(), &["task1", "task2"]);
        assert_eq!(
            decls.single_target(),
            Some(Target::Workflow("my_workflow".to_string()))
        );
    }

    #[test]
    fn multiple_tasks_no_workflow() {
        let wdl = r#"
            version 1.2
            task task1 {
                command { echo "1" }
            }
            task task2 {
                command { echo "2" }
            }
        "#;

        let decls = parse_wdl_declarations(wdl).unwrap();
        assert_eq!(decls.workflow(), None);
        assert_eq!(decls.tasks(), &["task1", "task2"]);
        assert_eq!(decls.single_target(), None);
    }

    #[test]
    fn no_workflow_or_task() {
        let wdl = r#"
            version 1.2
            # just a version, no declarations
        "#;

        let decls = parse_wdl_declarations(wdl).unwrap();
        assert_eq!(decls.workflow(), None);
        assert_eq!(decls.tasks(), &[] as &[String]);
        assert_eq!(decls.single_target(), None);
    }
}
