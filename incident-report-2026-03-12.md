# Incident Report

**Severity:** SEV2
**Status:** Investigating
**Date:** 2026-03-12
**Slack Thread:** https://aptos-org.slack.com/archives/C0ACWG0BGRX/p1773282697648049?thread_ts=1773282645.450869&cid=C0ACWG0BGRX

---

## Summary

Recent load testing reveals that mainnet throughput is capped at 750–800 TPS. Attempts to push beyond this rate result in unsustainable latency, impacting network performance and user experience.

---

## Impact

- **Throughput limitation:** Mainnet unable to sustain more than 750–800 TPS
- **Latency degradation:** Higher transaction rates cause latency to exceed acceptable thresholds
- **Node availability:** Several validator nodes remain down due to missed hotfix upgrades

---

## Timeline

| Time | Event |
|------|-------|
| TBD  | Initial detection of throughput limitations during load testing |
| TBD  | Incident declared as SEV2 |
| TBD  | Investigation initiated |

---

## Root Cause

Under investigation. Preliminary analysis points to:
- Performance bottlenecks under high load conditions
- Incomplete hotfix adoption across validator nodes
- Potential geographic latency factors

---

## Mitigation Actions

### In Progress
- [ ] Testing transaction submission to European validator nodes for potential latency improvements
- [ ] Reviewing hotfix adoption status and coordinating with node operators on missed upgrades
- [ ] Reviewing performance work items and timelines ([Notion link](https://www.notion.so/aptoslabs/Decibel-perf-work-items-H1-26-31a8b846eb7280269ae8d4292c1dbc62))

### Planned
- [ ] Implement identified performance optimizations
- [ ] Ensure all validator nodes are running the latest hotfix
- [ ] Develop longer-term scaling strategy for higher throughput

---

## Resolution Criteria

The incident will be considered resolved when:
1. **Throughput:** Mainnet sustains 1,000 TPS
2. **Latency:** Transaction latency maintained around 500 ms (target: 300–400 ms)

---

## Long-Term Actions

- Develop scaling plans to support higher order volumes
- Reduce end-to-end latency for improved user experience
- Establish monitoring and alerting for throughput/latency thresholds

---

## Lessons Learned

*To be completed post-incident*

---

## Attendees / Responders

*Add names of team members involved in incident response*

---

**Last Updated:** 2026-03-12
