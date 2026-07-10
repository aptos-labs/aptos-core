# Delegation Pool Governance Runbook

This runbook covers enabling delegation-pool governance and using delegated voter accounts to create, vote on, and resolve governance proposals.

## Enable Delegation-Pool Governance

Use `movement-migration/governance/enable_delegation_pool_governance.move` as a one-time core-resource-signer migration script. The script:

- enables `PARTIAL_GOVERNANCE_VOTING` and `DELEGATION_POOL_PARTIAL_GOVERNANCE_VOTING`;
- initializes global partial-voting records if missing;
- initializes delegation-pool governance records for each current mainnet delegated staking pool if missing;
- sets `min_voting_threshold` to `200,000,000 MOVE`;
- preserves the existing required proposer stake and voting duration;
- forces an epoch change so pending feature flags take effect.

The script takes the core resource signer and four delegation pool addresses:

```text
core_resources: &signer
pool_address_0: address
pool_address_1: address
pool_address_2: address
pool_address_3: address
```

As of June 29, 2026, the script initializes these mainnet delegated staking pools:

```text
0x1ef54ef84e7fb389095f83021755dd71bb51cbfbc8124a4349ec619f9d901f1f
0x830bfd0cd58b06dc938d409b6f3bc8ee97818ffcf9b32d714c068454afb644c7
0x39f116ee9ef048895bff51a5ce62229d153a6fe855798fa75810fd2b85008b9c
0xccba2d929183a642f64d10d27bae0947c112ed7f5427ca3c64a1f0dd0b4b76ea
```

If the delegated validator set changes before submission, pass the updated delegated staking pool addresses to the script. If the delegated validator count is no longer four, update the script arity before submitting the migration transaction.

Example execution by the core resource signer:

```bash
movement move run-script \
  --script-path movement-migration/governance/enable_delegation_pool_governance.move \
  --sender-account <CORE_RESOURCE_ADDRESS> \
  --private-key-file <CORE_RESOURCE_KEY_FILE> \
  --args address:0x1ef54ef84e7fb389095f83021755dd71bb51cbfbc8124a4349ec619f9d901f1f \
  --args address:0x830bfd0cd58b06dc938d409b6f3bc8ee97818ffcf9b32d714c068454afb644c7 \
  --args address:0x39f116ee9ef048895bff51a5ce62229d153a6fe855798fa75810fd2b85008b9c \
  --args address:0xccba2d929183a642f64d10d27bae0947c112ed7f5427ca3c64a1f0dd0b4b76ea \
  --url <NODE_URL> \
  --assume-yes
```

Do not run this migration after the core resource signer has been deprecated. This script intentionally uses the core resource signer to bootstrap delegated governance before proposal creation and voting are fully delegated.

## Create Delegated Voters

Create one voter account per delegation pool, then delegate each pool owner's voting power to that voter.

```bash
movement account create \
  --account <VOTER_ADDRESS> \
  --sender-account <POOL_OWNER_ADDRESS> \
  --private-key-file <POOL_OWNER_KEY_FILE> \
  --url <NODE_URL> \
  --assume-yes
```

```bash
movement move run \
  --function-id 0x1::delegation_pool::delegate_voting_power \
  --args address:<POOL_ADDRESS> address:<VOTER_ADDRESS> \
  --sender-account <POOL_OWNER_ADDRESS> \
  --private-key-file <POOL_OWNER_KEY_FILE> \
  --url <NODE_URL> \
  --assume-yes
```

Delegated voting power applies after the next lockup cycle. Verify before proposing or voting:

```bash
curl -fsS -H 'Content-Type: application/json' \
  -X POST <NODE_URL>/view \
  --data '{
    "function":"0x1::delegation_pool::calculate_and_update_voter_total_voting_power",
    "type_arguments":[],
    "arguments":["<POOL_ADDRESS>","<VOTER_ADDRESS>"]
  }'
```

## Create A Delegation-Pool Proposal

Create proposals through the delegation-pool governance command. The proposal signer must be a delegated voter with enough current voting power in the proposer pool.

```bash
movement governance delegation-pool propose \
  --delegation-pool-address <PROPOSER_POOL_ADDRESS> \
  --metadata-url <METADATA_URL> \
  --script-path <PROPOSAL_EXECUTION_SCRIPT.move> \
  --framework-local-dir aptos-move/framework/aptos-framework \
  --skip-fetch-latest-git-deps \
  --is-multi-step \
  --sender-account <VOTER_ADDRESS> \
  --private-key-file <VOTER_KEY_FILE> \
  --url <NODE_URL> \
  --assume-yes
```

The pool lockup must extend past `now + voting_duration_secs`; otherwise proposal creation aborts with `EINSUFFICIENT_STAKE_LOCKUP`.

## Review A Proposal

List recent governance proposals:

```bash
movement governance list-proposals \
  --url <NODE_URL>
```

View a submitted proposal before voting:

```bash
movement governance show-proposal \
  --proposal-id <PROPOSAL_ID> \
  --url <NODE_URL>
```

Reviewers should confirm:

- `metadata_verified` is `true`;
- the metadata title, description, source code URL, and discussion URL match the intended change;
- the `execution_hash` matches the expected script;
- the proposal expiration and current vote totals are acceptable.

Verify the submitted proposal against the local execution script:

```bash
movement governance verify-proposal \
  --proposal-id <PROPOSAL_ID> \
  --script-path <PROPOSAL_EXECUTION_SCRIPT.move> \
  --framework-local-dir aptos-move/framework/aptos-framework \
  --skip-fetch-latest-git-deps \
  --url <NODE_URL> \
  --assume-yes
```

Only vote after `verify-proposal` returns `verified: true` and the computed hash matches the on-chain `execution_hash`.

## Vote

Each delegated voter votes through its own delegation pool.

```bash
movement governance delegation-pool vote \
  --delegation-pool-address <POOL_ADDRESS> \
  --proposal-id <PROPOSAL_ID> \
  --yes \
  --sender-account <VOTER_ADDRESS> \
  --private-key-file <VOTER_KEY_FILE> \
  --url <NODE_URL> \
  --assume-yes
```

To verify vote totals:

```bash
curl -fsS -H 'Content-Type: application/json' \
  -X POST <NODE_URL>/view \
  --data '{
    "function":"0x1::voting::get_votes",
    "type_arguments":["0x1::governance_proposal::GovernanceProposal"],
    "arguments":["0x1","<PROPOSAL_ID>"]
  }'
```

## Resolve

Resolve after the proposal is in `SUCCEEDED` state. Early resolution requires the proposal's early-resolution vote threshold; otherwise wait until expiration.

```bash
curl -fsS -H 'Content-Type: application/json' \
  -X POST <NODE_URL>/view \
  --data '{
    "function":"0x1::voting::get_proposal_state",
    "type_arguments":["0x1::governance_proposal::GovernanceProposal"],
    "arguments":["0x1","<PROPOSAL_ID>"]
  }'
```

`1` means `SUCCEEDED`.

```bash
movement governance execute-proposal \
  --proposal-id <PROPOSAL_ID> \
  --script-path <PROPOSAL_EXECUTION_SCRIPT.move> \
  --framework-local-dir aptos-move/framework/aptos-framework \
  --skip-fetch-latest-git-deps \
  --url <NODE_URL> \
  --assume-yes
```

## Test Evidence

The following testnet transactions validate the flow used for this PR.

Migration transaction, using current testnet-only core resource signer flow to enable features, initialize `VotingRecordsV2`, and force epoch:

```text
0xc7ff3f97b59ddda40157c6b4b1b42e3560a25c74e50d224c5380e1a499611345
```

An earlier raw test proposal used SHA-256 of the script bytecode as the execution hash and could not be resolved because governance compares against the VM script hash. The replacement proposal below was created through the governance CLI and verified before voting.

Verified replacement proposal to remove non-delegated validator pool `0x4c724ccc61ec556afd6497ca469924eae0462b4a074b5d0fecfff3f4914f8dba`:

```text
proposal_id = 2
create proposal tx = 0x8495b783ef5a57cbe99dad9ae9d04dd58b7b78091526999c057bdb7fbe47278f
execution_hash = d2b7d808dab5908d96c62f18c3c63b12eef5d58ff0f4843b4863a5bc12ce356a
verify-proposal = true
```

Votes:

```text
0x6674e79ad1742fa88be36b5f575dbb7ddfe8e573fe0e8dc2bb3f21d10c399209
  pool 0xd963d9415c267862a1d72ccf9f26aa1c4091442f0bbc73cbcc59bfe734a00f7
  voter 0x3943756b811bb96cbf282b28b71620a8674b7123583d6065212bc102f03bd3bf
  yes votes 183044279128

0xc975d5fe0cdabf28c3a7bdfa26a86df8b5e628410d6bdc06785bfa23355e7a38
  pool 0x5fb9e7d3c56e2036549b1fd0c4ca3b920ca5f7a1efeb5a14f87a6ab8a39974ba
  voter 0x4202d872db4bcacb8c13d3ecceae71baf2ad5fa74a95d7ef58ddaebf5fbdf915
  yes votes 199382568060

0x05cd7fde71991c3b5efa43a037f1352492755d60159a80e096f6766888cfa5ab
  pool 0xbd7327ed25db839bd97d11f621afd7c6bd9ef7fa9e4a5044e02bc4e449ad18a6
  voter 0xe86a127d883dedaece5acc609a6b8807f596e2aabb6aca9a9e6e8faa768535d1
  yes votes 199493390394
```

Verified vote totals:

```text
yes = 581920237582
no = 0
```

Resolution:

```text
0x74745ac756c65847e78fdbe7b62d682230c0ca0ad146e0993ca7e80c5490154b
```

Post-resolution checks:

```text
proposal_id = 2
is_resolved = true
target_active = false
target_pending_inactive = true
active_validator_count = 4
```
