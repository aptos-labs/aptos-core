{{ frontmatter(name="move-init", description="Initialize Move workflow routing in the project CLAUDE.md") }}

## Task

Add Move workflow routing instructions to the project's CLAUDE.md so that
future prompts automatically delegate to the right agent.

### Steps

1. Find the Move package root by looking for `Move.toml` in the current
   directory or its parents.
2. If no `Move.toml` found, tell the user this is not a Move package and stop.
3. Read `CLAUDE.md` at the package root. If it does not exist, create it.
4. Check if the file already contains `<!-- move-flow-routing -->`.
   If yes, tell the user "Move workflow routing already configured" and stop.
5. Append the following block to the end of CLAUDE.md:

```
<!-- move-flow-routing -->
## Move Workflow Routing

When working with Move code in this package, use the appropriate workflow.
If the user asks to run something "in an agent" or "as a subagent", use
the Agent tool with the agent name. Otherwise, use the Skill tool with
the /skill-name to load the workflow into the current conversation.

- **Spec inference** (infer specs, generate specifications, WP analysis):
  Skill: `/move-inf`. Agent: `move-inf`.
- **Verification** (verify, prove, run prover, check specifications):
  Skill: `/move-prove`. Agent: `move-verify`.
- **Testing** (generate tests, unit tests, improve coverage):
  Skill: `/move-test`. Agent: `move-test`.
- **Fix compilation** (check errors, fix compilation, won't compile):
  Skill: `/move-check`. Agent: `move-check`.

Do not call `move_package_wp` or `move_package_verify` MCP tools directly —
always use the skill or agent which has the full workflow context.

For general Move development (writing code, reading code, explaining),
use the `/move` skill for language and tool references.
```

6. Tell the user that Move workflow routing has been configured.
