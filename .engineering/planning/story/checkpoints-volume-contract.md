---
format: aep.planning-md/3
id: story:checkpoints-volume-contract
kind: story
status: draft
title: A container run keeps its checkpoints on a documented home volume
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
- depends_on: story:container-image
revision: 1
---
## Outcome

The operating documentation states the volume contract for running cortex in containers: which
paths under `$CORTEX_HOME` must persist between runs, which may be lost, and what the home lock
needs of the volume. A run in a fresh container on that volume continues from the checkpoints the
previous run left.

## Why

Verified in this tree:

- Everything lives under the home, `$CORTEX_HOME` or `~/.local/share/cortex`
  (`website/docs/operating.md:11`): the registry with every source state and counters (`:15`), the
  lock (`:16`), each source seen state `state/<source>.json` (`:23`), the entity names offered to the
  model (`:24`), the batch directories (`:25`), the SQLite snapshots and their `state/` copies
  (`:26-27`) and `cortex.log` (`:28`).
- The lock is an exclusive lock on the file `cortex.lock` in the home (`src/home.rs:42-49`).
  Whether that lock holds across containers depends on the volume and its filesystem; not
  confirmed (nothing was run on a shared volume).
- For a `postgres` store, only the store moves off the disk (`website/docs/operating.md:20`); the
  checkpoints above stay on local disk.

The epic admits either outcome for this gap (the checkpoints move into the store, or the volume
contract is documented). This story takes the second, which adds no noun; moving the checkpoints
into the store would be its own story.

## Domain relations

- `Instance -> Source`, one-to-many, the instance owns its sources: `spec/domains/instance.yaml`,
  `cortex.instance.Instance` relation `sources` (`:541-545`).
- `Source -> SeenDocument`, one-to-many, the source owns what it has seen: the same document,
  `cortex.instance.Source` relation `seen` (`:568-572`). Its persistence is `state/<source>.json`
  (`website/docs/operating.md:23`); that mapping is inferred from the page, not stated in the spec.

## Work

- Classify each path of "Where things live" as must-persist, may-be-lost or per-run, and state what
  is lost when a may-be-lost path is (for example, snapshots of a `sqlite` store).
- State what the lock requires (one home per volume, the filesystems on which the lock holds, and
  that concurrent runs of one home rely on it), and how a scheduler avoids overlapping runs of one
  home (with `story:scheduler-one-shot-run` exit statuses).
- Add a test that runs two `cortex run` invocations as separate processes sharing only the home
  directory and checks the second continues from the first.
- Spec first: no new noun, command or field is expected; if the contract needs one (for example a
  setting that moves the lock), it goes into `spec/` first.

## Acceptance

- The operating page gains a section for containers that lists every path of "Where things live"
  with its persistence class.
- A second run of an unchanged source, in a fresh container from the release image on the same home
  volume, reports `documents_new: 0`.

## Not in this story

Moving the registry, seen state or lock into the EKR store or PostgreSQL; volume provisioning of any
cluster; the image itself (`story:container-image`).
