{# The candidate check: the closing step of inference and of manual verification. #}
{% if once(name="candidate_check") %}

### The candidate check

`{{ tool(name="move_spec_check") }}` is how a specification is
tested, whether it was inferred or written by hand. It compiles the package,
verifies the target, rejects a contract that weakens itself -- verification
disabled or skipped, a vacuous condition, a partial-abort pragma -- and
reports an obligation category the contract leaves uncovered. Give it
`package_path` and optionally `filter` to work on one module or function.

It verifies as part of accepting, so it replaces a closing call to
`{{ tool(name="move_package_verify") }}`: beforehand that repeats work the check
is about to do, and afterwards it re-proves what the check already proved. Call
the prover only to localize a failure the check has reported, over a narrower
`filter` than the check ran.

- Accepted: the requested scope is done. Stop and report.
- Rejected: the headline names which check failed, and the diagnostic lines that
  follow are `path:line: code: message`. A verification failure is worked with
  the verification workflow; a weakening code points at the clause that
  introduced it. A weakening you did not introduce -- a trusted boundary the
  project already had -- is reported, not removed.
- Unavailable: the prover could not run. That is not a verdict on the
  specification; report it as such.

The check reads the specification as written. Probing your own contract by
mutating a condition and re-checking is a legitimate way to convince yourself a
clause carries weight, at the cost of a verification round each time.

{% endif %}
