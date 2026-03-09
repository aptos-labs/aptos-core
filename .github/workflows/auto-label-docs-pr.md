---
on:
  pull_request:
    types: [opened, synchronize]

engine:
  id: claude

permissions:
  contents: read
  pull-requests: read

tools:
  github:
    toolsets: [pull_requests]

network: defaults

safe-outputs:
  add-labels:
    max: 2
  remove-labels:
    max: 2

---

# auto-label-docs-pr

Triage pull requests to determine whether they contain only documentation changes, and label them accordingly.

## Instructions

1. Retrieve the diff for the pull request that triggered this workflow.
2. Analyze every changed file in the diff:
   - A file is considered **doc-only** if it is one of:
     - A Markdown file (`.md`, `.mdx`)
     - A plain-text or reStructuredText file (`.txt`, `.rst`)
     - A file whose diff contains **only comment changes** (lines beginning with `//`, `#`, `*`, `///`, `--`, etc., or block-comment delimiters) and no changes to actual code logic.
   - A file is **not** doc-only if it contains any addition, removal, or modification of non-comment, non-documentation lines (e.g., source code logic, configuration values, build scripts, test assertions, etc.).
   - A file is a **workflow file** if its path matches `.github/workflows/**`.
3. Apply or remove the `doc-change` label:
   - If **every** changed file is doc-only: add the label `doc-change`.
   - If **any** changed file is not doc-only: remove the label `doc-change` (if currently applied).
4. Apply or remove the `gh-workflow` label:
   - If **any** changed file is a workflow file: add the label `gh-workflow`.
   - If **no** changed file is a workflow file: remove the label `gh-workflow` (if currently applied).

## Notes

- Be conservative: when in doubt about whether a line is a comment or code, treat it as code (not doc-only).
- Only add or remove the labels `doc-change` and `gh-workflow` — do not touch any other labels.
- The two labels are independent: a PR can have both, either, or neither.
