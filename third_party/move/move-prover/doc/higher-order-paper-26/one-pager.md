## X Article Draft (expanded, ~700 words)

### Verifying Move in the Age of AI-Driven Attacks

I've been heads-down at Aptos over the past few months revamping and extending the Move Prover, and the timing isn't an accident. AI is rapidly reshaping blockchain security on both sides of the line — and the defensive side has a lot of catching up to do.

**The offensive trend**

If you've been watching the security research feed over the past 18 months, the pattern is unmistakable. LLM-driven auditing pipelines — fine-tuned models, agentic exploit-discovery loops, automated invariant fuzzers wrapped around AI hypothesis generation — are now mature enough that they routinely surface exploitable bugs in code that human auditors had already cleared. White-hat write-ups appear weekly; black-hat use is harder to count but presumably tracks the same curve. Several high-profile DeFi incidents over the past year had AI fingerprints, either in the exploit-reconnaissance phase or in the cross-chain laundering that followed.

The economic asymmetry is brutal: a marginal exploit costs the attacker an API call; a marginal audit costs the defender an engineer-week of highly skilled work. The fix isn't to outspend the offense on AI cycles — it's to build defenses that are mathematically airtight. That is what formal verification has always offered: a proof against a precise specification is not vulnerable to "the model tried a clever input." Either the property holds for all inputs, or you have a concrete counterexample.

**What we built**

Over the last few months we landed four substantial additions to the Move Prover:

1. **An MCP service and Claude plugin.** The prover is now a tool that an AI coding agent (Claude Code, in our case) can call directly. Writing and proving specifications becomes a real back-and-forth with an AI that has live access to the verifier, not an offline guess.

2. **Specification inference, combining mechanical analysis with AI.** Writing specifications by hand has long been the main friction in formal verification. Our analyzer mechanically derives the routine parts of a specification from the code itself; the AI handles the high-level properties developers actually care about — things like "the protocol can never become insolvent," "fees can only accumulate," "no one can withdraw more than they deposited." Together they significantly cut the work of writing specifications from scratch.

3. **Verifiable first-class functions (dynamic dispatch).** Move now lets developers pass functions as values — a pluggable pricing curve in an AMM, a strategy stored inside a vault, a callback registered for an event. These patterns are common in modern smart contracts but historically hard to verify. We extended the specification language and the prover so that dynamic dispatch is handled natively: developers reason about *what* a passed-in function does, without committing to a particular implementation. At the caller side, any concrete instantiations are verified against the required contract.

4. **Proof hints and AI skills for generating them.** Sometimes the prover gets stuck on a tricky property — typically when nonlinear arithmetic is involved. We added a way to give the prover a nudge in the specification language, and trained an AI skill that proposes those nudges automatically and refines them until verification goes through.

Two papers cover the underlying work: *Formal Verification of Imperative First-Class Functions in Move* ([arXiv:2605.10007](https://arxiv.org/abs/2605.10007)) and *Combining Mechanical and Agentic Specification Inference for Move* ([arXiv:2605.10005](https://arxiv.org/abs/2605.10005)).

**Where this is heading: Decibel**

The next major target is Decibel — Aptos's high-throughput on-chain perpetual trading engine. Decibel is exactly the kind of system where formal verification pays for itself. Order matching, margin and liquidation logic, funding rates, oracle handling, fee accrual, settlement accounting — every one of these is a place where a subtle bug can drain a vault or mis-settle a position, and every one is something the prover should be checking on every PR before the code ships. The four pieces above are what promises to make that practical at Decibel's scale.

AI is making both sides of the security equation faster. On the defensive side, the substrate we've built lets us pair a mathematically rigorous prover with an AI that can do the tedious half of formal verification — and that combination is what makes verifying a system like Decibel realistic rather than aspirational.

— Wolfgang, Aptos Labs
