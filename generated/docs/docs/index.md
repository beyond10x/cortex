<!--
generated from cortex v1
model digest b4ec307bf35577cc40141d0d9ecfa84be945b1cfc5886af1ad638bfa53af8d64
contract digest 38571540d03e174471671e428e08bfba610dca6caa7e6de6077494ad0f35ae34
do not edit: regenerate with `ess generate`
-->

# cortex v1

Spin up knowledge brains from a spec. Each instance is one EKR store fed on a schedule by the data sources its spec connects: web pages found by search or crawled from listed sites, records of any Connectors operation, and local files. Secrets never pass through cortex; a source names a Connectors connection and only invokes operations through it.

## The system as a graph

```mermaid
flowchart TB
    subgraph who["who may ask"]
        who0["cortex.ask.Asker"]
        who1["cortex.instance.Operator"]
        who2["cortex.instance.Scheduler"]
    end
    subgraph unit0["cortex"]
        cmd1["cortex.instance.AddSource"]
        cmd2["cortex.instance.AdoptInstance"]
        cmd3["cortex.instance.ApplySchemaProposals"]
        cmd4["cortex.instance.ApproveSchemaProposals"]
        cmd5["cortex.instance.CreateInstance"]
        cmd6["cortex.instance.EnableSource"]
        cmd7["cortex.instance.MeasureQuality"]
        cmd8["cortex.instance.ProposeSchemaChanges"]
        cmd9["cortex.instance.RecordFailure"]
        cmd10["cortex.instance.RejectSchemaProposals"]
        cmd11["cortex.instance.RemoveInstance"]
        cmd12["cortex.instance.RestoreSnapshot"]
        cmd13["cortex.instance.RunSource"]
        cmd14["cortex.instance.UpdateInstance"]
        evt1["cortex.instance.InstanceAdopted"]
        evt2["cortex.instance.InstanceCreated"]
        evt3["cortex.instance.InstanceRemoved"]
        evt4["cortex.instance.InstanceUpdated"]
        evt5["cortex.instance.QualityMeasured"]
        evt6["cortex.instance.RunFailed"]
        evt8["cortex.instance.SchemaChangesProposed"]
        evt9["cortex.instance.SchemaProposalsApplied"]
        evt10["cortex.instance.SchemaProposalsDecided"]
        evt11["cortex.instance.SnapshotRestored"]
        evt12["cortex.instance.SourceAdded"]
        evt13["cortex.instance.SourceDisabled"]
        evt14["cortex.instance.SourceEnabled"]
        evt16["cortex.instance.SourceRan"]
    end
    subgraph unit1["cortex-ask"]
        cmd0["cortex.ask.Ask"]
        evt0["cortex.ask.Answered"]
    end
    subgraph unowned["owned by no component"]
        evt7["cortex.instance.RunSkipped"]
        evt15["cortex.instance.SourceFetched"]
    end
    who0 -->|"may invoke"| cmd0
    who1 -->|"may invoke"| cmd1
    who1 -->|"may invoke"| cmd2
    who1 -->|"may invoke"| cmd3
    who1 -->|"may invoke"| cmd4
    who1 -->|"may invoke"| cmd5
    who1 -->|"may invoke"| cmd6
    who1 -->|"may invoke"| cmd7
    who1 -->|"may invoke"| cmd8
    who1 -->|"may invoke"| cmd10
    who1 -->|"may invoke"| cmd11
    who1 -->|"may invoke"| cmd12
    who1 -->|"may invoke"| cmd14
    who2 -->|"may invoke"| cmd9
    who2 -->|"may invoke"| cmd13
    cmd0 -->|"answered"| evt0
    cmd1 -->|"added"| evt12
    cmd2 -->|"adopted"| evt1
    cmd3 -->|"applied"| evt9
    cmd4 -->|"approved"| evt10
    cmd5 -->|"partial"| evt2
    cmd5 -->|"created"| evt2
    cmd6 -->|"enabled"| evt14
    cmd7 -->|"measured"| evt5
    cmd8 -->|"proposed"| evt8
    cmd8 -->|"reviewed"| evt8
    cmd8 -->|"applied"| evt8
    cmd9 -->|"disabled"| evt13
    cmd9 -->|"counted"| evt6
    cmd10 -->|"rejected"| evt10
    cmd11 -->|"removed"| evt3
    cmd12 -->|"restored"| evt11
    cmd13 -->|"disabled"| evt7
    cmd13 -->|"fetched"| evt15
    cmd13 -->|"ran"| evt16
    cmd14 -->|"updated"| evt4
```

A command is accepted by the component that owns its context, emits the events one of its outcomes declares, and a dashed edge is a binding carrying an event into the next command. Design §9 begins one step earlier, at the actor who invokes the first command, and so does this graph: a solid edge out of an actor is a grant, and an actor drawn with no edge at all may invoke nothing — which is something the model says, not an arrow somebody forgot.

## Bounded contexts

- **[Answers](domains/cortex-ask.md)** (`cortex.ask`) — The answer service of an instance: `cortex ask serve` answers a question over the instance's store as a stream of events. It reads the store only through the EKR MCP reader (`ekr mcp`) and writes nothing to it; its model is the instance's model gateway, spoken to as an OpenAI-compatible endpoint. An answer is streamed, not stored. 10 types, no entities, no views, one command, one event, five errors and one actor.
- **[Instances](domains/cortex-instance.md)** (`cortex.instance`) — Instances of a knowledge brain and the data sources that feed them. An instance owns one EKR store, created from the instance spec's seed. A source fetches documents on its own schedule, keeps those that are new or changed, has a model extract them into an EKR extraction document and applies it to the instance's store. 75 types, three entities, two views, 14 commands, 16 events, 30 errors and two actors.

## Components

A component is a unit of ownership, not a deployment. How many of each runs, and what each needs, is [the topology](topology.md).

**`cortex`** — The operator's command line: create, update, run and remove instances, and switch a source off and on. A systemd user timer per source calls the same binary. It owns [`cortex.instance`](domains/cortex-instance.md). It accepts `cortex.instance.AddSource`, `cortex.instance.AdoptInstance`, `cortex.instance.ApplySchemaProposals`, `cortex.instance.ApproveSchemaProposals`, `cortex.instance.CreateInstance`, `cortex.instance.EnableSource`, `cortex.instance.MeasureQuality`, `cortex.instance.ProposeSchemaChanges`, `cortex.instance.RecordFailure`, `cortex.instance.RejectSchemaProposals`, `cortex.instance.RemoveInstance`, `cortex.instance.RestoreSnapshot`, `cortex.instance.RunSource` and `cortex.instance.UpdateInstance`. It publishes `cortex.instance.InstanceAdopted`, `cortex.instance.InstanceCreated`, `cortex.instance.InstanceRemoved`, `cortex.instance.InstanceUpdated`, `cortex.instance.QualityMeasured`, `cortex.instance.RunFailed`, `cortex.instance.SchemaChangesProposed`, `cortex.instance.SchemaProposalsApplied`, `cortex.instance.SchemaProposalsDecided`, `cortex.instance.SnapshotRestored`, `cortex.instance.SourceAdded`, `cortex.instance.SourceDisabled`, `cortex.instance.SourceEnabled` and `cortex.instance.SourceRan`.

**`cortex-ask`** — The answer service, `cortex ask serve --spec <instance spec>`: a long-running process that answers `POST /ask` as a stream of events over the instance's store, read through the EKR MCP reader. The batch component `cortex` serves no port; this one serves the spec's `ask.listen`. It owns [`cortex.ask`](domains/cortex-ask.md). It accepts `cortex.ask.Ask`. It publishes `cortex.ask.Answered`.

## The other pages

| page | what is on it |
|---|---|
| [Answers](domains/cortex-ask.md) | the `cortex.ask` vocabulary: its types, entities, views, commands, events, errors and actors |
| [Instances](domains/cortex-instance.md) | the `cortex.instance` vocabulary: its types, entities, views, commands, events, errors and actors |
| [Interactions](interactions.md) | every binding, with what it guarantees and what happens when it fails |
| [Type crossings](crossings.md) | every conversion this system permits, and the reason someone gave for it |
| [Topology](topology.md) | what each component needs in order to run |


---

Generated from cortex v1 · model digest `b4ec307bf35577cc40141d0d9ecfa84be945b1cfc5886af1ad638bfa53af8d64` · contract digest `38571540d03e174471671e428e08bfba610dca6caa7e6de6077494ad0f35ae34`. Do not edit this file; change the specification and regenerate it with `ess generate`.
