# USDK stablecoin design

Scope: this document covers only the `stablecoin` example package. It records the
security model the module implements so that reviewers can check the code against
an explicit set of intended guarantees.

## Roles

| Role | Capabilities |
| --- | --- |
| `master_minter` | Grants/revokes the minter role and sets per-minter mint allowances. Cannot mint. |
| minter | Mints up to its remaining allowance. Burns only its own balance. |
| `pauser` | Pauses and unpauses value movement. |
| `denylister` | Freezes/unfreezes accounts. |
| `confiscator` | Burns balances held by an arbitrary account. |

The master minter deliberately has no implicit mint authority. To mint it must
grant itself an allowance, which is an auditable on-chain event.

## Issuance control

Each minter holds a finite allowance, decremented atomically on every mint. A
compromised minter key can issue at most its remaining allowance. Allowances are
stored in an address-keyed table, so authorization and allowance lookup do not
scan a list and cannot be made arbitrarily expensive by adding minters.

## Role rotation

Every privileged role rotates in two steps: the current holder proposes a
successor, and the successor accepts. A proposal alone grants nothing, so a
mistyped address cannot strand a role. The current holder may replace or clear a
pending proposal at any time. Both steps emit events.

## Pause semantics

Pause stops value movement — mint, burn, and transfer — while preserving the
operations needed to respond to an incident.

Allowed while paused: minter revocation, allowance *decreases*, denylisting and
undenylisting, confiscation, and role proposal/acceptance.

Blocked while paused: minting, burning, transfers, granting new minters, and
allowance *increases*. A compromised master-minter key therefore cannot expand
issuing authority during an emergency.

## Denylist enforcement

Denylisting sets the frozen flag on the account's primary store. Two independent
layers enforce it:

1. The framework rejects withdrawals from a frozen store before dispatch runs.
2. The module's `withdraw`/`deposit` hooks reject any store whose direct owner or
   root owner is denylisted.

Layer 2 is what prevents a denylisted account from escaping enforcement by
parking funds in a store owned by an intermediate object it controls, since the
framework's own check accepts indirect ownership. All stores for this asset are
untransferable, so a store cannot be moved out from under the check.

## Non-goals

- No delegated-transfer/approval flow. The previous `transfer_from` was removed
  rather than left disabled: its approvals were replayable because the nonce was
  the account sequence number, which the function never advanced. A replacement
  needs a contract-owned nonce, a deadline, domain separation, and cancellation.
- No global supply cap. Issuance is bounded by per-minter allowances instead.
- Initialization stays an explicit `entry` call rather than `init_module`.
