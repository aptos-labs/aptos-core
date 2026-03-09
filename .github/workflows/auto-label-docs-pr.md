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
  remove-labels:

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
3. If **every** changed file is doc-only:
   - Add the label `doc-change` to the pull request.
4. If **any** changed file is not doc-only:
   - Remove the label `doc-change` from the pull request (if it is currently applied).

## Notes

- Be conservative: when in doubt about whether a line is a comment or code, treat it as code (not doc-only).
- Do not add or remove any labels other than `doc-change`.
