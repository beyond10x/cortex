---
format: aep.planning-md/3
id: vision:self-updating-instances
kind: vision
status: draft
title: Knowledge instances that keep themselves current from their configured sources
revision: 1
---
## Objective

Anyone can start a knowledge instance from one spec file — seed schema, model settings and the data
sources it reads — and the instance keeps itself current: on a schedule it fetches through
Connectors, extracts what is new into its own EKR store, and serves that store over MCP. An instance
is defined by which sources are configured and connected, not by a topic.

## Measured by

- An instance runs unattended on its schedule for a week with no manual step.
- A document already applied is never applied twice, and costs no model call.
- The store links what it extracts: facts carry edges, not only properties.
