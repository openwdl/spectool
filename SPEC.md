# WDL Conformance Test Configuration Specification

This document specifies the test configuration format for WDL conformance tests embedded in markdown files as processed by `spectool`.

## Overview

Conformance tests are embedded in markdown files (such as the WDL specification) using HTML `<details>` elements. Each test may include an optional "Test config" section containing a JSON object that configures test execution behavior.

## Test Configuration Format

The test configuration is a JSON object that appears in a fenced code block following the "Test config:" header within the `<p>` section of a `<details>` element.

Example:

<details>
  <summary>
  Example: hello.wdl

  ```wdl
  version 1.2
  workflow hello {
    ...
  }
  ```
  </summary>
  <p>
  Test config:

  ```json
  {
    "ignore": false,
    "fail": false,
    "capabilities": ["gpu"]
  }
  ```
  </p>
</details>

## Configuration Parameters

All configuration parameters are optional. If not specified, they use their default values.

### `target`

The name of the workflow or task to execute.

- **Type**: String (optional)
- **Default**: Automatically inferred from WDL content
- **Constraints**:
  - May ONLY be specified when there are multiple tasks and no input JSON (cannot be inferred)
  - MUST NOT be specified when target can be inferred from input JSON prefix
  - MUST NOT be specified when there is only one workflow or task

**Inference rules:**
1. If the WDL contains a workflow, that workflow is executed
2. If the WDL contains no workflow and a single task, that task is executed
3. If the WDL contains no workflow and multiple tasks, the target is inferred from the input JSON prefix (e.g., `"my_task.input1"` → run `my_task`)
4. If the WDL contains no workflow, multiple tasks, and no input JSON, `target` MUST be specified

**Example:**
```json
{
  "target": "my_specific_task"
}
```

### `ignore`

Whether to skip this test entirely.

- **Type**: Boolean
- **Default**: `false`
- **Description**: If `true`, the test is not executed and not counted in test results.

**Example:**
```json
{
  "ignore": true
}
```

### `fail`

Whether the test is expected to fail.

- **Type**: Boolean
- **Default**: `false`
- **Description**: If `true`, a failed execution is treated as a successful test, and a successful execution is treated as a test failure. This is used to test error handling and validation.

**Example:**
```json
{
  "fail": true
}
```

### `return_code`

The expected return code(s) for test execution.

- **Type**: Integer, array of integers, or the special string `"*"`
- **Default**: `"*"` (any return code is allowed)
- **Description**: Specifies the expected return code(s) when a test completes. The special value `"*"` indicates that any return code is acceptable. This is particularly useful for tests marked with `fail: true` to ensure they fail with the expected return code.

**Examples:**
```json
{"return_code": 0}
{"return_code": 1}
{"return_code": [1, 2, 3]}
{"return_code": "*"}
```

### `exclude_outputs`

Output parameters to exclude from validation.

- **Type**: Array of strings
- **Default**: `[]` (empty array)
- **Description**: Specifies output parameter names that should be ignored when comparing expected and actual outputs. This is useful for outputs that may vary between executions (e.g., timestamps, temporary file paths, non-deterministic values).

**Example:**
```json
{
  "exclude_outputs": ["timestamp", "tmp_file"]
}
```

### `capabilities`

Runtime capabilities required by the test.

- **Type**: Array of capability enums
- **Default**: `[]` (empty array)
- **Allowed values**: `"cpu"`, `"memory"`, `"gpu"`, `"disks"`, `"allow_nested_inputs"`
- **Description**: Specifies runtime resources or capabilities that the test requires. Tests are only executed if ALL required capabilities are provided via the `--capabilities` command-line flag. Tests with unsatisfied capabilities are skipped entirely.

The allowed capability values are:
- `"cpu"` - requires specific CPU resources
- `"memory"` - requires specific memory resources
- `"gpu"` - requires GPU hardware
- `"disks"` - requires specific disk resources
- `"allow_nested_inputs"` - allows setting nested workflow/task inputs at runtime

**Examples:**
```json
{"capabilities": ["gpu"]}
{"capabilities": ["cpu", "memory"]}
```

**Command-line usage:**
```bash
spectool test --capabilities gpu,memory <command>
```

**Validation:**
Unknown capability strings are rejected at parse time with an error.

## Complete Example

<details>
  <summary>
  Example: gpu_accelerated.wdl

  ```wdl
  version 1.2
  task gpu_task {
    input {
      File data
      Int threads = 4
    }

    command <<<
      gpu-process ~{data} --threads ~{threads}
    >>>

    requirements {
      gpu: true
      memory: "16 GB"
    }

    output {
      File result = stdout()
    }
  }
  ```
  </summary>
  <p>
  Example input:

  ```json
  {
    "gpu_task.data": "input.txt",
    "gpu_task.threads": 8
  }
  ```

  Example output:

  ```json
  {
    "gpu_task.result": "expected_output.txt"
  }
  ```

  Test config:

  ```json
  {
    "capabilities": ["gpu", "memory"],
    "return_code": 0
  }
  ```
  </p>
</details>

## Implementation Notes

### Type Safety

The configuration JSON must be parsed with strict type checking. Unknown fields are rejected to catch typos and ensure forward compatibility.

### Default Value Resolution

Default values are used when configuration parameters are not explicitly specified.

### Target Inference and Validation

The `target` field has strict validation rules to prevent ambiguity:

1. If `target` can be inferred but is still provided → **ERROR**
2. If `target` is needed (multiple tasks, no input) but not provided → **ERROR**
3. If `target` is provided when only one workflow/task exists → **ERROR**

### Capability Checking

Tests are skipped (not executed, not counted) when required capabilities are not provided via command line. The test framework must validate that all capability strings are recognized enum values, rejecting unknown capabilities at parse time.
