<!--
generated from cortex v1
model digest b979825d9ba3898417b16740c98b19baa65c45db0467e0e887c9ec8c35b8a194
contract digest af1fe44bd7162114b5142156736e336d7e7f76c9c135606857f141c8aa77ff7a
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
        cmd18["cortex.instance.RunSource"]
        cmd19["cortex.instance.UpdateInstance"]
        evt6["cortex.instance.InstanceAdopted"]
        evt7["cortex.instance.InstanceCreated"]
        evt8["cortex.instance.InstanceRemoved"]
        evt9["cortex.instance.InstanceUpdated"]
        evt10["cortex.instance.QualityMeasured"]
        evt11["cortex.instance.RunFailed"]
        evt13["cortex.instance.SchemaChangesProposed"]
        evt14["cortex.instance.SchemaProposalsApplied"]
        evt15["cortex.instance.SchemaProposalsDecided"]
        evt16["cortex.instance.SnapshotRestored"]
        evt17["cortex.instance.SourceAdded"]
        evt18["cortex.instance.SourceDisabled"]
        evt19["cortex.instance.SourceEnabled"]
        evt21["cortex.instance.SourceRan"]
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
        evt12["cortex.instance.RunSkipped"]
        evt20["cortex.instance.SourceFetched"]
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
    who3 -->|"may invoke"| cmd19
    who4 -->|"may invoke"| cmd14
    who4 -->|"may invoke"| cmd18
    cmd0 -->|"answered"| evt1
    cmd1 -->|"changed"| evt5
    cmd1 -->|"cast"| evt5
    cmd2 -->|"closed"| evt3
    cmd3 -->|"opened"| evt4
    cmd4 -->|"purged"| evt2
    cmd5 -->|"recorded"| evt0
    cmd6 -->|"added"| evt17
    cmd7 -->|"adopted"| evt6
    cmd8 -->|"applied"| evt14
    cmd9 -->|"approved"| evt15
    cmd10 -->|"partial"| evt7
    cmd10 -->|"created"| evt7
    cmd11 -->|"enabled"| evt19
    cmd12 -->|"measured"| evt10
    cmd13 -->|"proposed"| evt13
    cmd13 -->|"reviewed"| evt13
    cmd13 -->|"applied"| evt13
    cmd14 -->|"disabled"| evt18
    cmd14 -->|"counted"| evt11
    cmd15 -->|"rejected"| evt15
    cmd16 -->|"removed"| evt8
    cmd17 -->|"restored"| evt16
    cmd18 -->|"disabled"| evt12
    cmd18 -->|"fetched"| evt20
    cmd18 -->|"ran"| evt21
    cmd19 -->|"updated"| evt9
```

A command is accepted by the component that owns its context, emits the events one of its outcomes declares, and a dashed edge is a binding carrying an event into the next command. Design §9 begins one step earlier, at the actor who invokes the first command, and so does this graph: a solid edge out of an actor is a grant, and an actor drawn with no edge at all may invoke nothing — which is something the model says, not an arrow somebody forgot.

## Bounded contexts

- **[Answers](domains/cortex-ask.md)** (`cortex.ask`) — The answer service of an instance: `cortex ask serve` answers a question over the instance's store as a stream of events. It reads the store only through the EKR MCP reader (`ekr mcp`) and writes nothing to it; its model is the instance's model gateway, spoken to as an OpenAI-compatible endpoint. Every answer is streamed and kept as an answer record, with the votes of its readers; an answer voted down, failed or slow is queued for review. The records are cortex's own, in `ask.sqlite` in the instance directory, never in the store, and are purged after the spec's `retention_days`. 16 types, three entities, one view, six commands, six events, nine errors and three actors.
- **[Instances](domains/cortex-instance.md)** (`cortex.instance`) — Instances of a knowledge brain and the data sources that feed them. An instance owns one EKR store, created from the instance spec's seed. A source fetches documents on its own schedule, keeps those that are new or changed, has a model extract them into an EKR extraction document and applies it to the instance's store. 75 types, three entities, two views, 14 commands, 16 events, 30 errors and two actors.

## Components

A component is a unit of ownership, not a deployment. How many of each runs, and what each needs, is [the topology](topology.md).

**`cortex`** — The operator's command line: create, update, run and remove instances, and switch a source off and on. A systemd user timer per source calls the same binary. It owns [`cortex.instance`](domains/cortex-instance.md). It accepts `cortex.instance.AddSource`, `cortex.instance.AdoptInstance`, `cortex.instance.ApplySchemaProposals`, `cortex.instance.ApproveSchemaProposals`, `cortex.instance.CreateInstance`, `cortex.instance.EnableSource`, `cortex.instance.MeasureQuality`, `cortex.instance.ProposeSchemaChanges`, `cortex.instance.RecordFailure`, `cortex.instance.RejectSchemaProposals`, `cortex.instance.RemoveInstance`, `cortex.instance.RestoreSnapshot`, `cortex.instance.RunSource` and `cortex.instance.UpdateInstance`. It publishes `cortex.instance.InstanceAdopted`, `cortex.instance.InstanceCreated`, `cortex.instance.InstanceRemoved`, `cortex.instance.InstanceUpdated`, `cortex.instance.QualityMeasured`, `cortex.instance.RunFailed`, `cortex.instance.SchemaChangesProposed`, `cortex.instance.SchemaProposalsApplied`, `cortex.instance.SchemaProposalsDecided`, `cortex.instance.SnapshotRestored`, `cortex.instance.SourceAdded`, `cortex.instance.SourceDisabled`, `cortex.instance.SourceEnabled` and `cortex.instance.SourceRan`.

**`cortex-ask`** — The answer service, `cortex ask serve --spec <instance spec>`: a long-running process that answers `POST /ask` as a stream of events over the instance's store, read through the EKR MCP reader. The batch component `cortex` serves no port; this one serves the spec's `ask.listen`. It keeps every answer with its votes and a review queue in `ask.sqlite` in the instance directory, and purges them after the spec's `retention_days`. It owns [`cortex.ask`](domains/cortex-ask.md). It accepts `cortex.ask.Ask`, `cortex.ask.CastVote`, `cortex.ask.CloseReview`, `cortex.ask.OpenReview`, `cortex.ask.PurgeExpired` and `cortex.ask.RecordAnswer`. It publishes `cortex.ask.AnswerRecorded`, `cortex.ask.Answered`, `cortex.ask.Purged`, `cortex.ask.ReviewClosed`, `cortex.ask.ReviewOpened` and `cortex.ask.Voted`.

## The other pages

| page | what is on it |
|---|---|
| [Answers](domains/cortex-ask.md) | the `cortex.ask` vocabulary: its types, entities, views, commands, events, errors and actors |
| [Instances](domains/cortex-instance.md) | the `cortex.instance` vocabulary: its types, entities, views, commands, events, errors and actors |
| [Interactions](interactions.md) | every binding, with what it guarantees and what happens when it fails |
| [Type crossings](crossings.md) | every conversion this system permits, and the reason someone gave for it |
| [Topology](topology.md) | what each component needs in order to run |


---

Generated from cortex v1 · model digest `b979825d9ba3898417b16740c98b19baa65c45db0467e0e887c9ec8c35b8a194` · contract digest `af1fe44bd7162114b5142156736e336d7e7f76c9c135606857f141c8aa77ff7a`. Do not edit this file; change the specification and regenerate it with `ess generate`.
