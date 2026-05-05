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
