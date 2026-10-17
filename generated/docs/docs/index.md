<!--
generated from cortex v1
model digest 35445edbfb924602bf5ee4c9b32cce8f1c4d409e8e17675c8b68b61dbaf6bec5
contract digest 6cc21d245bef94a86dddf5ce3002dec278709616b6c49500ec2220a65697e5a7
do not edit: regenerate with `ess generate`
-->

# cortex v1

Spin up knowledge brains from a spec. Each instance is one EKR store fed on a schedule by the data sources its spec connects: web pages found by search or crawled from listed sites, records of any Connectors operation, and local files. Secrets never pass through cortex; a source names a Connectors connection and only invokes operations through it.

## The system as a graph

```mermaid
flowchart TB
    subgraph who["who may ask"]
        who0["cortex.ask.Asker"]
        who1["cortex.ask.Reviewer"]
        who2["cortex.ask.Service"]
        who3["cortex.instance.Operator"]
        who4["cortex.instance.Scheduler"]
    end
    subgraph unit0["cortex"]
        cmd6["cortex.instance.AddSource"]
        cmd7["cortex.instance.AdoptInstance"]
        cmd8["cortex.instance.ApplySchemaProposals"]
        cmd9["cortex.instance.ApproveSchemaProposals"]
        cmd10["cortex.instance.CreateInstance"]
        cmd11["cortex.instance.EnableSource"]
        cmd12["cortex.instance.MeasureQuality"]
        cmd13["cortex.instance.ProposeSchemaChanges"]
        cmd14["cortex.instance.RecordFailure"]
        cmd15["cortex.instance.RejectSchemaProposals"]
        cmd16["cortex.instance.RemoveInstance"]
        cmd17["cortex.instance.RestoreSnapshot"]
        cmd18["cortex.instance.RunEval"]
        cmd19["cortex.instance.RunSource"]
        cmd20["cortex.instance.UpdateInstance"]
        evt6["cortex.instance.EvalRan"]
        evt7["cortex.instance.InstanceAdopted"]
        evt8["cortex.instance.InstanceCreated"]
        evt9["cortex.instance.InstanceRemoved"]
        evt10["cortex.instance.InstanceUpdated"]
        evt11["cortex.instance.QualityMeasured"]
        evt12["cortex.instance.RunFailed"]
        evt14["cortex.instance.SchemaChangesProposed"]
        evt15["cortex.instance.SchemaProposalsApplied"]
        evt16["cortex.instance.SchemaProposalsDecided"]
        evt17["cortex.instance.SnapshotRestored"]
        evt18["cortex.instance.SourceAdded"]
        evt19["cortex.instance.SourceDisabled"]
        evt20["cortex.instance.SourceEnabled"]
        evt22["cortex.instance.SourceRan"]
    end
    subgraph unit1["cortex-ask"]
        cmd0["cortex.ask.Ask"]
        cmd1["cortex.ask.CastVote"]
        cmd2["cortex.ask.CloseReview"]
        cmd3["cortex.ask.OpenReview"]
        cmd4["cortex.ask.PurgeExpired"]
        cmd5["cortex.ask.RecordAnswer"]
        evt0["cortex.ask.AnswerRecorded"]
        evt1["cortex.ask.Answered"]
        evt2["cortex.ask.Purged"]
        evt3["cortex.ask.ReviewClosed"]
        evt4["cortex.ask.ReviewOpened"]
        evt5["cortex.ask.Voted"]
    end
    subgraph unowned["owned by no component"]
        evt13["cortex.instance.RunSkipped"]
        evt21["cortex.instance.SourceFetched"]
    end
    who0 -->|"may invoke"| cmd0
    who0 -->|"may invoke"| cmd1
    who1 -->|"may invoke"| cmd2
    who2 -->|"may invoke"| cmd3
    who2 -->|"may invoke"| cmd4
    who2 -->|"may invoke"| cmd5
    who3 -->|"may invoke"| cmd6
    who3 -->|"may invoke"| cmd7
    who3 -->|"may invoke"| cmd8
    who3 -->|"may invoke"| cmd9
    who3 -->|"may invoke"| cmd10
    who3 -->|"may invoke"| cmd11
    who3 -->|"may invoke"| cmd12
    who3 -->|"may invoke"| cmd13
    who3 -->|"may invoke"| cmd15
    who3 -->|"may invoke"| cmd16
    who3 -->|"may invoke"| cmd17
    who3 -->|"may invoke"| cmd18
    who3 -->|"may invoke"| cmd20
    who4 -->|"may invoke"| cmd14
    who4 -->|"may invoke"| cmd19
    cmd0 -->|"answered"| evt1
    cmd1 -->|"changed"| evt5
    cmd1 -->|"cast"| evt5
    cmd2 -->|"closed"| evt3
    cmd3 -->|"opened"| evt4
    cmd4 -->|"purged"| evt2
    cmd5 -->|"recorded"| evt0
    cmd6 -->|"added"| evt18
    cmd7 -->|"adopted"| evt7
    cmd8 -->|"applied"| evt15
    cmd9 -->|"approved"| evt16
    cmd10 -->|"partial"| evt8
    cmd10 -->|"created"| evt8
    cmd11 -->|"enabled"| evt20
    cmd12 -->|"measured"| evt11
    cmd13 -->|"proposed"| evt14
    cmd13 -->|"reviewed"| evt14
    cmd13 -->|"applied"| evt14
    cmd14 -->|"disabled"| evt19
    cmd14 -->|"counted"| evt12
    cmd15 -->|"rejected"| evt16
    cmd16 -->|"removed"| evt9
    cmd17 -->|"restored"| evt17
    cmd18 -->|"evaluated"| evt6
    cmd19 -->|"disabled"| evt13
    cmd19 -->|"fetched"| evt21
    cmd19 -->|"ran"| evt22
    cmd20 -->|"updated"| evt10
```

A command is accepted by the component that owns its context, emits the events one of its outcomes declares, and a dashed edge is a binding carrying an event into the next command. Design §9 begins one step earlier, at the actor who invokes the first command, and so does this graph: a solid edge out of an actor is a grant, and an actor drawn with no edge at all may invoke nothing — which is something the model says, not an arrow somebody forgot.

## Bounded contexts

- **[Answers](domains/cortex-ask.md)** (`cortex.ask`) — The answer service of an instance: `cortex ask serve` answers a question over the instance's store as a stream of events. It reads the store only through the EKR MCP reader (`ekr mcp`) and writes nothing to it; its model is the instance's model gateway, spoken to as an OpenAI-compatible endpoint. Every answer is streamed and kept as an answer record, with the votes of its readers; an answer voted down, failed or slow is queued for review. The records are cortex's own, never in the store: in `ask.sqlite` in the instance directory on a `sqlite` store, in a PostgreSQL schema of their own on a `postgres` one; they are purged after the spec's `retention_days`. 17 types, three entities, one view, six commands, six events, nine errors and three actors.
- **[Instances](domains/cortex-instance.md)** (`cortex.instance`) — Instances of a knowledge brain and the data sources that feed them. An instance owns one EKR store, created from the instance spec's seed. A source fetches documents on its own schedule, keeps those that are new or changed, has a model extract them into an EKR extraction document and applies it to the instance's store. 79 types, four entities, two views, 15 commands, 17 events, 32 errors and two actors.

## Components

A component is a unit of ownership, not a deployment. How many of each runs, and what each needs, is [the topology](topology.md).

**`cortex`** — The operator's command line: create, update, run and remove instances, and switch a source off and on. A systemd user timer per source calls the same binary. It owns [`cortex.instance`](domains/cortex-instance.md). It accepts `cortex.instance.AddSource`, `cortex.instance.AdoptInstance`, `cortex.instance.ApplySchemaProposals`, `cortex.instance.ApproveSchemaProposals`, `cortex.instance.CreateInstance`, `cortex.instance.EnableSource`, `cortex.instance.MeasureQuality`, `cortex.instance.ProposeSchemaChanges`, `cortex.instance.RecordFailure`, `cortex.instance.RejectSchemaProposals`, `cortex.instance.RemoveInstance`, `cortex.instance.RestoreSnapshot`, `cortex.instance.RunEval`, `cortex.instance.RunSource` and `cortex.instance.UpdateInstance`. It publishes `cortex.instance.EvalRan`, `cortex.instance.InstanceAdopted`, `cortex.instance.InstanceCreated`, `cortex.instance.InstanceRemoved`, `cortex.instance.InstanceUpdated`, `cortex.instance.QualityMeasured`, `cortex.instance.RunFailed`, `cortex.instance.SchemaChangesProposed`, `cortex.instance.SchemaProposalsApplied`, `cortex.instance.SchemaProposalsDecided`, `cortex.instance.SnapshotRestored`, `cortex.instance.SourceAdded`, `cortex.instance.SourceDisabled`, `cortex.instance.SourceEnabled` and `cortex.instance.SourceRan`.

**`cortex-ask`** — The answer service, `cortex ask serve --spec <instance spec>`: a long-running process that answers `POST /ask` as a stream of events over the instance's store, read through the EKR MCP reader. The batch component `cortex` serves no port; this one serves the spec's `ask.listen`. It keeps every answer with its votes and a review queue in `ask.sqlite` in the instance directory, or, for an instance on a `postgres` store, in a PostgreSQL schema of their own through a records process launched by Connectors (`cortex ask records`), and purges them after the spec's `retention_days`. It owns [`cortex.ask`](domains/cortex-ask.md). It accepts `cortex.ask.Ask`, `cortex.ask.CastVote`, `cortex.ask.CloseReview`, `cortex.ask.OpenReview`, `cortex.ask.PurgeExpired` and `cortex.ask.RecordAnswer`. It publishes `cortex.ask.AnswerRecorded`, `cortex.ask.Answered`, `cortex.ask.Purged`, `cortex.ask.ReviewClosed`, `cortex.ask.ReviewOpened` and `cortex.ask.Voted`.

## The other pages

| page | what is on it |
|---|---|
| [Answers](domains/cortex-ask.md) | the `cortex.ask` vocabulary: its types, entities, views, commands, events, errors and actors |
| [Instances](domains/cortex-instance.md) | the `cortex.instance` vocabulary: its types, entities, views, commands, events, errors and actors |
| [Interactions](interactions.md) | every binding, with what it guarantees and what happens when it fails |
| [Type crossings](crossings.md) | every conversion this system permits, and the reason someone gave for it |
| [Topology](topology.md) | what each component needs in order to run |


---

Generated from cortex v1 · model digest `35445edbfb924602bf5ee4c9b32cce8f1c4d409e8e17675c8b68b61dbaf6bec5` · contract digest `6cc21d245bef94a86dddf5ce3002dec278709616b6c49500ec2220a65697e5a7`. Do not edit this file; change the specification and regenerate it with `ess generate`.
