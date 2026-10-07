<!--
generated from cortex v1
model digest baa6b022b0cc1c204b3bf7ee74c0509918a69054e4dfa82fba89ab9da19c30e9
contract digest 81310f63a3f86377583f8de316f242c2e516e5001c9a0c53de91cb771a7aac31
do not edit: regenerate with `ess generate`
-->

# cortex v1

Spin up knowledge brains from a spec. Each instance is one EKR store fed on a schedule by the data sources its spec connects: web pages found by search or crawled from listed sites, records of any Connectors operation, and local files. Secrets never pass through cortex; a source names a Connectors connection and only invokes operations through it.

## The system as a graph

```mermaid
flowchart TB
    subgraph who["who may ask"]
        who0["cortex.instance.Operator"]
        who1["cortex.instance.Scheduler"]
    end
    subgraph unit0["cortex"]
        cmd0["cortex.instance.AddSource"]
        cmd1["cortex.instance.AdoptInstance"]
        cmd2["cortex.instance.CreateInstance"]
        cmd3["cortex.instance.EnableSource"]
        cmd4["cortex.instance.MeasureQuality"]
        cmd5["cortex.instance.ProposeSchemaChanges"]
        cmd6["cortex.instance.RecordFailure"]
        cmd7["cortex.instance.RemoveInstance"]
        cmd8["cortex.instance.RestoreSnapshot"]
        cmd9["cortex.instance.RunSource"]
        cmd10["cortex.instance.UpdateInstance"]
        evt0["cortex.instance.InstanceAdopted"]
        evt1["cortex.instance.InstanceCreated"]
        evt2["cortex.instance.InstanceRemoved"]
        evt3["cortex.instance.InstanceUpdated"]
        evt4["cortex.instance.QualityMeasured"]
        evt5["cortex.instance.RunFailed"]
        evt6["cortex.instance.SchemaChangesProposed"]
        evt7["cortex.instance.SnapshotRestored"]
        evt8["cortex.instance.SourceAdded"]
        evt9["cortex.instance.SourceDisabled"]
        evt10["cortex.instance.SourceEnabled"]
        evt11["cortex.instance.SourceRan"]
    end
    who0 -->|"may invoke"| cmd0
    who0 -->|"may invoke"| cmd1
    who0 -->|"may invoke"| cmd2
    who0 -->|"may invoke"| cmd3
    who0 -->|"may invoke"| cmd4
    who0 -->|"may invoke"| cmd5
    who0 -->|"may invoke"| cmd7
    who0 -->|"may invoke"| cmd8
    who0 -->|"may invoke"| cmd10
    who1 -->|"may invoke"| cmd6
    who1 -->|"may invoke"| cmd9
    cmd0 -->|"added"| evt8
    cmd1 -->|"adopted"| evt0
    cmd2 -->|"partial"| evt1
    cmd2 -->|"created"| evt1
    cmd3 -->|"enabled"| evt10
    cmd4 -->|"measured"| evt4
    cmd5 -->|"proposed"| evt6
    cmd5 -->|"applied"| evt6
    cmd6 -->|"disabled"| evt9
    cmd6 -->|"counted"| evt5
    cmd7 -->|"removed"| evt2
    cmd8 -->|"restored"| evt7
    cmd9 -->|"ran"| evt11
    cmd10 -->|"updated"| evt3
```

A command is accepted by the component that owns its context, emits the events one of its outcomes declares, and a dashed edge is a binding carrying an event into the next command. Design §9 begins one step earlier, at the actor who invokes the first command, and so does this graph: a solid edge out of an actor is a grant, and an actor drawn with no edge at all may invoke nothing — which is something the model says, not an arrow somebody forgot.

## Bounded contexts

- **[Instances](domains/cortex-instance.md)** (`cortex.instance`) — Instances of a knowledge brain and the data sources that feed them. An instance owns one EKR store, created from the instance spec's seed. A source fetches documents on its own schedule, keeps those that are new or changed, has a model extract them into an EKR extraction document and applies it to the instance's store. 66 types, three entities, two views, 11 commands, 12 events, 22 errors and two actors.

## Components

A component is a unit of ownership, not a deployment. How many of each runs, and what each needs, is [the topology](topology.md).

**`cortex`** — The operator's command line: create, update, run and remove instances, and switch a source off and on. A systemd user timer per source calls the same binary. It owns [`cortex.instance`](domains/cortex-instance.md). It accepts `cortex.instance.AddSource`, `cortex.instance.AdoptInstance`, `cortex.instance.CreateInstance`, `cortex.instance.EnableSource`, `cortex.instance.MeasureQuality`, `cortex.instance.ProposeSchemaChanges`, `cortex.instance.RecordFailure`, `cortex.instance.RemoveInstance`, `cortex.instance.RestoreSnapshot`, `cortex.instance.RunSource` and `cortex.instance.UpdateInstance`. It publishes `cortex.instance.InstanceAdopted`, `cortex.instance.InstanceCreated`, `cortex.instance.InstanceRemoved`, `cortex.instance.InstanceUpdated`, `cortex.instance.QualityMeasured`, `cortex.instance.RunFailed`, `cortex.instance.SchemaChangesProposed`, `cortex.instance.SnapshotRestored`, `cortex.instance.SourceAdded`, `cortex.instance.SourceDisabled`, `cortex.instance.SourceEnabled` and `cortex.instance.SourceRan`.

## The other pages

| page | what is on it |
|---|---|
| [Instances](domains/cortex-instance.md) | the `cortex.instance` vocabulary: its types, entities, views, commands, events, errors and actors |
| [Interactions](interactions.md) | every binding, with what it guarantees and what happens when it fails |
| [Type crossings](crossings.md) | every conversion this system permits, and the reason someone gave for it |
| [Topology](topology.md) | what each component needs in order to run |


---

Generated from cortex v1 · model digest `baa6b022b0cc1c204b3bf7ee74c0509918a69054e4dfa82fba89ab9da19c30e9` · contract digest `81310f63a3f86377583f8de316f242c2e516e5001c9a0c53de91cb771a7aac31`. Do not edit this file; change the specification and regenerate it with `ess generate`.
