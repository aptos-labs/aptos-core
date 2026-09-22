# Public Etna evaluation results

Etna source is private. The archives in this directory contain aggregate
reports and tables only. They intentionally omit per-run diffs, diagnostics,
transcripts, event streams, source trees, and any other artifact that can carry
source context.

Before publication, build and audit archives with `harness.publication`. The
scanner enforces a top-level aggregate-file allowlist, validates internal
checksums and tar structure, and rejects private source paths and unified diffs.
The repository test suite scans every archive in this directory.
