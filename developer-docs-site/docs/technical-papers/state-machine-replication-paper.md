---
title: "State Machine Replication"
slug: "state-machine-replication-paper"
hidden: false
sidebar_position: 1
---
import PublicationLink from "@site/src/components/PublicationLink";

***Note to readers: On December 1, 2020, the Libra Association was renamed to Aptos Association. This report was modified
in April 2020 to incorporate updates to the Libra payment system as found in the White Paper v2.0. Features of the
project as implemented may differ based on regulatory approvals or other considerations, and may evolve over time.***

## Abstract

This report describes the Aptos Byzantine Fault Tolerance (AptosBFT) algorithmic core and discusses next steps in its
production. The consensus protocol is responsible for forming agreement on ordering and finalizing transactions among a
configurable set of validators. AptosBFT maintains safety against network asynchrony and even if at any particular
configuration epoch, a threshold of the participants are Byzantine.

AptosBFT is based on HotStuff, a recent protocol that leverages several decades of scientific advances in Byzantine Fault
Tolerance (BFT) and achieves the strong scalability and security properties required by internet settings. Several novel
features distinguish AptosBFT from HotStuff. AptosBFT incorporates a novel round synchronization mechanism that provides
bounded commit latency under synchrony. It introduces a nil-block vote that allows proposals to commit despite having
faulty leaders. It encapsulates the correct behavior by participants in a “tcb”-able module, allowing it to run within a
secure hardware enclave that reduces the attack surface on participants.

AptosBFT can reconfigure itself, by embedding configuration-change commands in the sequence. A new configuration epoch
may change everything from the validator set to the protocol itself.

### Downloads
<PublicationLink
    image="/img/docs/state-machine-pdf.png"
    doc_link="/papers/aptos-consensus-state-machine-replication-in-the-aptos-blockchain/2021-08-17.pdf"
    title="State Machine Replication in the Aptos Blockchain"
/>
