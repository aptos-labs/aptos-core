{# Tool-independent WP background, shared by every inference tactic. #}
{% if once(name="wp_concepts") %}

### Weakest-precondition reasoning

Weakest-precondition (WP) reasoning works backward from returns, aborts, calls,
and state updates to characterize the initial states for each behavior. A loop
is represented by its invariant; values the invariant does not constrain are
effectively arbitrary after the loop.

{% endif %}
