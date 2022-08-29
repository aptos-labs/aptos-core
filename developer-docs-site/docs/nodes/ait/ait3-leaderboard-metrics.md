---
title: "AIT-3 Leaderboard Metrics"
slug: "ait-3-leaderboard-metrics"
---

# AIT-3 Leaderboard Metrics

This document explains how the rewards for AIT-3 validator are evaluated and displayed on the [AIT-3 Validator Status](https://community.staging.gcp.aptosdev.com/leaderboard/it3?sort=-rewards_growth) page. 

## How are rewards calculated

:::tip Staking documentation
For a backgrounder on staking with explanations of epoch, rewards and how to join and leave validator set, see the [Staking](/concepts/staking.md). 
:::

- For the AIT-3 the epoch duration is 7200 seconds, i.e., two hours.
- An epoch starts with a finalized validator set. During the epoch, only validators in this validator set will vote. 
- During the epoch, following the process described in [Validation on the Aptos blockchain](/concepts/staking#validation-on-the-aptos-blockchain), a validator is selected as a leader to make a proposal. Because the validator set is unchanged during the course of an epoch, this will result in a validator being selected multiple times as a leader in an epoch.
-  On successful proposals, i.e., proposals achieving the quorum consensus, the leaders earn rewards based on their stake and on the reward rate that is configured on-chain. The reward rate is the same for every validator.
-  If all the proposals in an epoch achieve quorum consensus, a validator earns the maximum reward for the epoch. **Rewards are given only to the leader validators, and not to the voters.**
-  On failed proposals, i.e., a proposal that did not achieve the quorum consensus, the leaders do not earn any reward for that proposal.
-  If all the proposals in an epoch fail, a validator earns zero rewards for that epoch.

## Reward performance

- The reward performance of a validator is calculated as a % of reward earned by the validator out of the maximum reward earning opportunity i.e., `(rewards earned across the epochs)/(maximum reward opportunity across the epochs)`. **This is a cumulative metric across all the epochs.**
- A validator can improve their performance by improving their proposal success rate.

## Last epoch performance

The LAST EPOCH PERFORMANCE column shown on the leaderboard is reported as `(number of successful proposals)/(number of total proposal opportunities)`.
- This metric gives you an early indicator if your performance is slowly reducing.
- You can see the JSON dump (link on the leaderboard) to see the performance across all the epochs.
- On mouse hover, you can see the last epoch for the validator.

## Governance votes

The GOVERNANCE VOTES column shown on the leaderboard is reported as `(governance proposals voted on)/(total governance votes)`.




        
        
      