use similar::TextDiff;

const MARKER: &str = "# tether-workflow-version: 1";
const WORKFLOW_PATH: &str = ".github/workflows/tether.yml";

/// Returns the workflow Tether installs in a GitHub repository.
#[must_use]
pub fn workflow_template() -> &'static str {
    r#"# tether-workflow-version: 1
name: tether
run-name: tether ${{ inputs.request_id }}

on:
  workflow_dispatch:
    inputs:
      command:
        description: Command to run
        required: true
        type: string
      request_id:
        description: Tether request id
        required: true
        type: string
      setup:
        description: Runtime setup
        required: false
        default: ''
        type: string

permissions:
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        if: inputs.setup == 'python'
      - uses: actions/setup-node@v4
        if: inputs.setup == 'node'
      - uses: dtolnay/rust-toolchain@stable
        if: inputs.setup == 'rust'
      - uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: '17'
        if: inputs.setup == 'java'
      - name: Run command
        env:
          TETHER_COMMAND: ${{ inputs.command }}
        run: bash -euo pipefail -c "$TETHER_COMMAND"
"#
}

/// A proposed workflow file and its unified diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowProposal {
    /// Repository-relative destination.
    pub path: String,
    /// Complete file content.
    pub content: String,
    /// Whether the file does not exist yet.
    pub is_new: bool,
    /// Unified diff from the current content.
    pub diff: String,
}

/// Detects the first requested runtime from repository root files.
#[must_use]
pub fn detect_setup(paths: &[&str]) -> Option<&'static str> {
    let has = |name: &str| paths.contains(&name);
    if has("pyproject.toml")
        || paths.iter().any(|path| {
            path.starts_with("requirements")
                && !path.contains('/')
                && std::path::Path::new(path)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
        })
    {
        return Some("python");
    }
    if has("package.json") {
        return Some("node");
    }
    if has("Cargo.toml") {
        return Some("rust");
    }
    if ["build.gradle", "build.gradle.kts", "pom.xml"]
        .iter()
        .any(|name| has(name))
    {
        return Some("java");
    }
    None
}

/// Returns whether an existing file is the current Tether workflow.
#[must_use]
pub fn workflow_is_current(existing: Option<&str>) -> bool {
    existing.is_some_and(|value| value.starts_with(MARKER) && value == workflow_template())
}

/// Proposes the current workflow when the repository file is absent or stale.
#[must_use]
pub fn propose_workflow(existing: Option<&str>) -> Option<WorkflowProposal> {
    if workflow_is_current(existing) {
        return None;
    }
    let old = existing.unwrap_or_default();
    let diff = TextDiff::from_lines(old, workflow_template())
        .unified_diff()
        .header(
            "a/.github/workflows/tether.yml",
            "b/.github/workflows/tether.yml",
        )
        .to_string();
    Some(WorkflowProposal {
        path: WORKFLOW_PATH.into(),
        content: workflow_template().into(),
        is_new: existing.is_none(),
        diff,
    })
}
