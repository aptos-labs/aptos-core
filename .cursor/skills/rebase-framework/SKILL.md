---
name: rebase-framework
description: Use when rebasing a branch that modifies the Aptos Move framework and git reports merge conflicts in cached-packages artifacts (`head.mrb` or generated SDK builder `.rs` files under `aptos-move/framework/cached-packages/src/`). Also use when a rebase stops on a binary conflict in `head.mrb`, or when a CI job complains that "Cached framework artifacts are out-of-date" after a rebase.
---

# Rebase with framework cached-packages conflicts

## Overview

`aptos-move/framework/cached-packages/src/head.mrb` is a **serialized binary** of the compiled Move framework. Sibling files (`aptos_framework_sdk_builder.rs`, etc.) in the same directory are **code-generated** from the framework. Neither can be merge-resolved sensibly — they must be **rebuilt** from the framework sources at the conflict point.

**The rule:** don't try to 3-way merge cached-packages artifacts. Take one side to unblock git, finish the rebase, then regenerate **once** at the end with `scripts/cargo_build_aptos_cached_packages.sh`.

## When to use

Conflict markers / git output that should trigger this skill:

- `CONFLICT (content): Merge conflict in aptos-move/framework/cached-packages/src/head.mrb`
- `warning: Cannot merge binary files: ...head.mrb`
- `CONFLICT` in any of `aptos-move/framework/cached-packages/src/*.rs`
- CI error after rebase: `ERROR: Cached framework artifacts are out-of-date.`

If the conflict is in **framework source** (e.g., `aptos-move/framework/aptos-framework/sources/*.move`), resolve that conflict the normal way first — the cached-packages step comes after.

## Procedure

1. **Resolve real source conflicts first.** Walk the rebase, hand-resolving any `.move`, `.rs`, or `Cargo.toml` conflicts as usual. For cached-packages artifacts, **don't** try to merge — just unblock git:
   ```bash
   # Take the incoming/current side; contents will be regenerated anyway.
   git checkout --ours  aptos-move/framework/cached-packages/src/head.mrb
   git checkout --ours  aptos-move/framework/cached-packages/src/*.rs
   git add              aptos-move/framework/cached-packages/src/head.mrb \
                        aptos-move/framework/cached-packages/src/*.rs
   git rebase --continue
   ```
   `--ours` vs `--theirs` doesn't matter for these files — they're about to be overwritten. Pick whichever lets git proceed.

2. **Continue the rebase normally** through the remaining commits. If cached-packages conflicts reappear on later commits, repeat step 1. **Do not** rebuild between commits — it's slow and wasted work.

3. **After the rebase finishes,** regenerate the artifacts once from the final tree:
   ```bash
   scripts/cargo_build_aptos_cached_packages.sh
   ```
   This runs `cargo run --profile=ci -p aptos-framework -- update-cached-packages` and formats the generated `.rs` files.

4. **Amend the regenerated artifacts into the framework commit** (so history stays clean — bytecode change lives with the source change that caused it):
   ```bash
   git status aptos-move/framework/cached-packages/src/
   # Identify the commit that touched framework sources. Then:
   git add aptos-move/framework/cached-packages/src/head.mrb \
           aptos-move/framework/cached-packages/src/*.rs
   # If the framework-changing commit is HEAD:
   git commit --amend --no-edit
   # Otherwise, fixup + autosquash:
   git commit --fixup=<framework-commit-sha>
   GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash <framework-commit-sha>^
   ```

5. **Verify** before pushing:
   ```bash
   scripts/cargo_build_aptos_cached_packages.sh --check
   ```
   This re-runs the build and fails if any artifact would change — the same gate CI uses.

## Quick reference

| Situation | Action |
|-----------|--------|
| Binary conflict in `head.mrb` mid-rebase | `git checkout --ours head.mrb && git add ...` — don't rebuild yet |
| Conflict in generated `.rs` under `cached-packages/src/` | Same — take one side, rebuild at end |
| Multiple framework-changing commits in the rebase | Resolve each as above; rebuild **once** after `rebase --continue` finishes |
| Conflict in framework `.move` sources | Resolve normally; rebuild step still runs at the end |
| CI says artifacts out-of-date after push | Run the script, amend/fixup into the framework commit, force-push |

## Common mistakes

- **Trying to merge `head.mrb` by hand or with a mergetool.** It's a binary blob — there's nothing to merge. Always regenerate.
- **Rebuilding after every conflicting commit during the rebase.** Wastes minutes per commit. Rebuild once at the end.
- **Committing the regenerated artifacts as a separate "rebuild framework" commit.** Reviewers expect the bytecode delta to live with the Move source change. Use `--amend` or `--fixup` to keep them together.
- **Skipping `--check` before pushing.** CI runs the same check; failing it round-trips the PR.
- **Forgetting the generated `.rs` files.** The script regenerates both `head.mrb` **and** `aptos-move/framework/cached-packages/src/*.rs`. Stage both.

## Related

- `aptos-move/framework/cached-packages/README.md` — upstream docs for the artifact.
- Repo `CLAUDE.md` note: "After modifying Move code in `aptos-move/framework/`: `cargo build -p aptos-cached-packages`" — that's the per-edit workflow; this skill covers the rebase variant.
