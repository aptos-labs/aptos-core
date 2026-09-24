# Evaluation result archives

The archives in this directory use the compact aggregate publication format.
Each bundle contains supported reports and tables plus an internal checksum
manifest.

Build and audit archives with `harness.publication`. The scanner enforces the
top-level member allowlist and validates checksums, size limits, and tar
structure. Workspace diffs and other raw per-run artifacts are omitted. The
repository test suite scans every archive in this directory.
