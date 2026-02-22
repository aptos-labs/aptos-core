---
name: move-dev
description: Move development assistant
skills: [move]
---

{#
This agent exists as an intermediary so workflow skills (move-check, move-prove)
can preload the `move` skill into a forked context. Skills only support
`context: fork` + `agent: <name>`; the `skills` field is agent-only.
#}

# Move Development Agent

You assist with Move smart contract development on Aptos using {{ platform_display }}.
