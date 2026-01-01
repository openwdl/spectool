# `spectool`

A conformance testing tool for WDL (Workflow Description Language) execution engines.

## Overview

`spectool` extracts conformance tests from the WDL specification and runs them against WDL execution engines to verify compliance with the specification. Tests are extracted from the WDL `SPEC.md` file and compiled into executable WDL files that can be run against any WDL engine.

## Building

```bash
# Build `spectool` at `target/release/spectool`.
cargo build --release

# Install `spectool` to the path.
cargo install --path .
```

## Usage

`spectool` has a number of options that allow it to call all major WDL execution
engines. You can use `spectool` like so,

```bash
spectool test "sprocket run ~{path} ~{input} -e ~{target}" --redirect-stdout
```

The command template supports the following substitutions:

- `~{path}` — path to the WDL test file
- `~{input}` — path to the input JSON file
- `~{output}` — path to the output JSON file
- `~{target}` — name of the workflow or task to execute

### Common Options

**Specify the WDL specification directory:**

```bash
spectool test "sprocket run ~{path} ~{input} -e ~{target}" --redirect-stdout -s ~/openwdl/wdl
```

**Save compiled tests to a directory:**

```bash
spectool test "sprocket run ~{path} ~{input} -e ~{target}" --redirect-stdout -c ./conformance-tests
```

**Filter tests by name:**

```bash
# Run only tests matching "array"
spectool test "sprocket run ~{path} ~{input} -e ~{target}" --include array 

# Exclude tests matching "fail"
spectool test "sprocket run ~{path} ~{input} -e ~{target}" --exclude fail 
```

**Inject a different WDL version:**

```bash
# Replace version 1.2 with version development (useful for Cromwell)
spectool test "cromwell run ~{path} -i ~{input}" --inject-wdl-version development --redirect-stdout
```

**Transform output JSON before validation:**

```bash
# Extract .outputs field from the engine's output (useful for MiniWDL)
spectool test "miniwdl run ~{path} -i ~{input}" --output-selector '.outputs' --redirect-stdout
```

**Test with specific capabilities:**

```bash
spectool test "..." --capabilities optional_inputs,optional_outputs 
```

## Example Workflows

### Testing Sprocket

```bash
spectool test "sprocket run ~{path} ~{input} -e ~{target}" --redirect-stdout
```

### Testing Cromwell

```bash
spectool test \
  "cromwell run ~{path} -i ~{input}" \
  --inject-wdl-version development \
  --redirect-stdout
```

### Testing MiniWDL

```bash
spectool test \
  "miniwdl run ~{path} -i ~{input}" \
  --output-selector '.outputs' \
  --redirect-stdout
```

## License

This tool is made available to you under [the BSD 3-Clause License](./LICENSE).

Copyright (c) 2025, The OpenWDL Developers
