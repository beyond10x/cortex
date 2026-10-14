---
format: aep.planning-md/3
id: upstream-blocker:connectors-headless-custody
kind: upstream-blocker
status: open
title: Connectors has no credential custody without a desktop session
relations:
- blocks: story:headless-connectors-credentials
revision: 1
---
## What would clear it

A Connectors release with a credential custody that works with no desktop session (a read-only, owner-only mounted secret file or Kubernetes Secret), and a documented setup for a container. Today custody binds only to a qualified gnome-keyring daemon with an unlocked `login` collection on the user's session bus (Connectors `docs/local-secret-service.md`), which a container does not have.
