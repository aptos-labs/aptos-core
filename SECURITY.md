# Aptos Core Bug Bounty

## Reporting a Security Concern

**DO NOT CREATE AN ISSUE** to report a security problem.

Go to https://github.com/aptos-labs/aptos-core/security/advisories and open a vulnerability report. Send an email to [security@aptosfoundation.org](mailto:security@aptosfoundation.org) and provide your GitHub username. The team will triage the issue from there.

For security reasons, DO NOT include attachments or provide detail sufficient for exploitation regarding the security issue in this email. Instead, wait for the advisory to be created, and **provide any sensitive details in the private GitHub advisory**.

If you haven't done so already, please **enable two-factor authentation** in your GitHub account.

Send the email from an email domain that is less likely to get flagged for spam by gmail.

This is an actively monitored account, the team will quickly respond.

If you do not receive a response within 24 hours, please directly followup with the team in [Discord](https://discord.gg/aptosnetwork) by reaching out to anyone with the role “Aptos Labs”.

As above, please DO NOT include attachments or provide detail regarding the security issue in this email.

## Incident Response Process

1. Establish a new draft security advisory
    1. In response to an email to [security@aptosfoundation.org](mailto:security@aptosfoundation.org), a member of the Aptos Labs will create a new draft security advisory for the incident at [https://github.com/aptos-labs/aptos-core/security/policy](https://github.com/aptos-labs/aptos-core/security/policy).
    2. Add the reporter's GitHub account and relevant individuals to the draft security advisory.
    3. Respond to the reporter by email, sharing a link to the draft security advisory.
2. Responder should add appropriate content to the draft security advisory. To be eligible for a bug bounty, this includes:
    1. A clear description of the issue and the impacted areas.
    2. The code and the methodology to reproduce the underlying issue.
    3. Discussion of potential remediations.
3. Triage
    1. Validate the issue.
    2. Determine the criticality of the issue.
    3. If this is a bug and not a security issue, recommend to the submitter to create an issue.
4. Resolve by building new docker containers and binaries that contain the fix and pushing to affected entities.

## Bug Bounties

Aptos Foundation offers bounties for security reports. Reports will be validated against the GitHub repository branch labeled "mainnet" and no others. Moreover, any code within mainnet but not in an actively deployed network or in a position to be deployed in such an environment, such as experimental or actively developed code, are out of scope for the bug bounty program. In addition, if a bug discovered in mainnet is already fixed in main, it is out of scope. Production environments include the Aptos testnet and mainnet.

The scope includes code within

- [Aptos Core — main branch](https://github.com/aptos-labs/aptos-core/tree/main)
- [Move — Aptos branch](https://github.com/move-language/move/tree/aptos)

Aptos Foundation considers the following levels of severity:

### Critical — Up to $1,000,000 USD in APT tokens (locked for 12 months)

- Direct loss of funds to users or protocols with minimal preconditions, such as, Move type confusion.
- Vulnerabilities in the Proof of Stake system which directly compromise consensus.
- Unintended permanent chain split requiring hard fork (network partition requiring hardfork).
- Permanent freezing, burning, or modification of funds (fix requires hardfork).

### High — Up to $100,000 USD in APT tokens (locked for 12 months)

- Loss of funds with some pre-conditions, such as, halting the network.
- Interrupting blockchain progress or halting the network.

### Medium — Up to $25,000 USD in APT tokens (locked for 12 months)

- Denial of service issues which compromise the integrity or availability of the chain.
- Loss of funds with many preconditions.
- Ability to crash a production node with some pre-conditions.

## Payment of Bug Bounties

- Bounties are currently awarded on a rolling/weekly basis and paid out within 30 days upon receipt of successful KYC and payment contract.
- The APT/USD conversion rate used for payments is the market price of APT (denominated in USD) at 11:59 PM PST the day that both KYC and the payment contract are completed.
- The reference for this price is the Closing Price given by Coingecko.com on that date given here: [https://www.coingecko.com/en/coins/aptos/historical_data#panel](https://www.coingecko.com/en/coins/aptos/historical_data#panel)
- Bug bounties that are paid out in APT are paid to locked to the account provided by the reporter with a lockup expiring 12 months from the date of the delivery of APT.
- Multiple vulnerabilities of similar root cause will be paid out as one report.

## Duplicate Reports

Compensation for duplicate reports will be split among reporters with first to report taking priority using the following equation:

```
R: total reports
ri: report priority
bi: bounty share

bi = 2 ^ (R - ri) / ((2^R) - 1)
```

Where report priority derives from the set of integers beginning at 1, where the first reporter has `ri = 1`, the second reporter `ri = 2`, and so forth.

Note, reports that come in after the issue has been fully triaged and resolved will not be eligible for splitting.

## KYC Requirements

This bug bounty program is only open to individuals [outside the OFAC restricted countries](https://home.treasury.gov/policy-issues/financial-sanctions/sanctions-programs-and-country-information). Bug bounty hunters will be required to provide evidence that they are not a resident or citizen of these countries to be eligible for a reward. If the individual is a US person, tax information will be required, such as a W-9, in order to properly issue a 1099. Aptos requires KYC to be done for all bug bounty hunters submitting a report and wanting a reward. Form W-9 or Form W-8 is required for tax purposes. All bug bounty hunters are required to use Persona for KYC, links will be provided upon resolution of the issue. The collection of this information will be done by the Aptos Foundation.

If an impact can be caused to any other asset managed by Aptos that isn’t on this table but for which the impact is not inscope, you are encouraged to submit it for consideration by the project.

## Out of Scope

The following vulnerabilities are excluded from the rewards for this bug bounty program:

- Attacks that the reporter has already exploited themselves, leading to damage.
- Attacks requiring access to leaked keys/credentials.
- Internally known issues, duplicate issues, or issues that have already been made public; in such cases, proof of prior disclosure will be provided.
- Attacks that rely on social engineering or require physical access to the victim’s device.
- Information disclosure with minimal security impact (Ex: stack traces, path disclosure, directory listing, logs).
- Tab-nabbing.
- Vulnerabilities related to auto-fill web forms.
- Vulnerabilities only exploitable on out-of-date browsers or platforms.
- Attacks requiring physical access to the victim device.
- Incorrect data supplied by third party oracles.

### Blockchain:

- Basic economic governance attacks (e.g. 51% attack).
- Best practice critiques.
- Missing or incorrect data in events.
- Sybil attacks.
- Centralization risk.

### Smart Contracts:

- Incorrect data supplied by third-party oracles (not to exclude oracle manipulation/flash loan attacks; use of such methods to generate critical impacts remain in-scope for this program).
- Basic economic governance attacks (e.g., 51% attack).
- Lack of liquidity.
- Best practice critiques.
- Missing or incorrect data in events.
- Incorrect naming (but still correct data) in contracts.
- Minor rounding errors that don’t lead to substantial loss of funds.

## Exclusions

The following activities are prohibited by this bug bounty program:

- Any testing with mainnet or public testnet contracts; all testing should be done on [private testnets](https://aptos.dev/nodes/local-testnet/local-testnet-index/).
- Any testing with pricing oracles or third party smart contracts.
- Attempting to phish or otherwise use social engineering attacks against contributors, employees, and/or customers.
- Any testing with third-party systems and applications (e.g., browser extensions) as well as websites (e.g., SSO providers, advertising networks).
- Any denial of service attacks.
- Automated testing of services that generates significant amounts of traffic.
- Public disclosure of an unpatched vulnerability in an embargoed bounty.
