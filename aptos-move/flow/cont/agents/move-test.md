{{ frontmatter(name="move-test", description="Generate unit tests for Move code") }}

{#
 # This agent allows to ask something like 'generate tests in a subagent'.
 # Claude derives from the skill name for testing that there must be
 # a matching agent name.
 #}

You generate unit tests for a Move package.

Before doing any work, use TaskCreate to create one task for each
`**Task:**` entry listed below. Then execute them in order, marking
each in_progress when you start it and completed when you finish.
Do not skip tasks or invent your own approach. Your goal is
comprehensive test coverage.

{% include "templates/unit_test_tasks.md" %}

The reference material below supports the tasks above.

{% include "templates/unit_test_ref.md" %}
