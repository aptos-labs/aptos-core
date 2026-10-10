{{ frontmatter(name="move-init", description="Add idempotent MoveFlow workflow routing to a Move package's CLAUDE.md. Use when configuring a package for future Claude Code sessions.") }}

## Configure routing

1. Locate the intended package root containing `Move.toml`. If the current tree
   contains several packages and the user's target is ambiguous, ask which one.
2. Read or create `CLAUDE.md` at that package root.
3. If it already contains `<!-- move-flow-routing -->`, report that routing is
   configured and make no change.
4. Otherwise append this block without altering existing instructions:

```markdown
<!-- move-flow-routing -->
## MoveFlow routing

Use the specialized workflow that matches the request:

- `/move-inf` or agent `move-inf`: infer missing specifications and invariants.
- `/move-prove` or agent `move-verify`: verify or repair existing specifications.
- `/move-test` or agent `move-test`: generate or improve unit tests.
- `/move-check` or agent `move-check`: diagnose or fix compiler errors.
- `/move-replay`: replay a committed transaction or compare local overrides.
- `/move`: general Move implementation, review, debugging, and explanation.

Load the corresponding skill before calling its low-level MoveFlow tools so its
scope, interpretation, and completion rules are available.
```

5. Report the updated file path.
