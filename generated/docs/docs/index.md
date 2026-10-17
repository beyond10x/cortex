<!--
generated from cortex v1
model digest 590eec96e468842d47bbd36e564dfe80cfcb3f6ff17a1259e23fd89709ba10d7
contract digest a29e3a1337a53e4441a5e167da33ade01c37192a6adae9420ea33dbe05308d96
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
        cmd2["cortex.instance.ApplySchemaProposals"]
        cmd3["cortex.instance.ApproveSchemaProposals"]
        cmd4["cortex.instance.CreateInstance"]
        cmd5["cortex.instance.EnableSource"]
        cmd6["cortex.instance.MeasureQuality"]
        cmd7["cortex.instance.ProposeSchemaChanges"]
        cmd8["cortex.instance.RecordFailure"]
        cmd9["cortex.instance.RejectSchemaProposals"]
        cmd10["cortex.instance.RemoveInstance"]
        cmd11["cortex.instance.RestoreSnapshot"]
        cmd12["cortex.instance.RunSource"]
        cmd13["cortex.instance.UpdateInstance"]
        evt0["cortex.instance.InstanceAdopted"]
        evt1["cortex.instance.InstanceCreated"]
        evt2["cortex.instance.InstanceRemoved"]
        evt3["cortex.instance.InstanceUpdated"]
        evt4["cortex.instance.QualityMeasured"]
        evt5["cortex.instance.RunFailed"]
        evt7["cortex.instance.SchemaChangesProposed"]
        evt8["cortex.instance.SchemaProposalsApplied"]
        evt9["cortex.instance.SchemaProposalsDecided"]
        evt10["cortex.instance.SnapshotRestored"]
        evt11["cortex.instance.SourceAdded"]
        evt12["cortex.instance.SourceDisabled"]
        evt13["cortex.instance.SourceEnabled"]
        evt15["cortex.instance.SourceRan"]
    end
    subgraph unowned["owned by no component"]
        evt6["cortex.instance.RunSkipped"]
        evt14["cortex.instance.SourceFetched"]
    end
    who0 -->|"may invoke"| cmd0
    who0 -->|"may invoke"| cmd1
    who0 -->|"may invoke"| cmd2
    who0 -->|"may invoke"| cmd3
    who0 -->|"may invoke"| cmd4
    who0 -->|"may invoke"| cmd5
    who0 -->|"may invoke"| cmd6
    who0 -->|"may invoke"| cmd7
    who0 -->|"may invoke"| cmd9
    who0 -->|"may invoke"| cmd10
    who0 -->|"may invoke"| cmd11
    who0 -->|"may invoke"| cmd13
    who1 -->|"may invoke"| cmd8
    who1 -->|"may invoke"| cmd12
    cmd0 -->|"added"| evt11
    cmd1 -->|"adopted"| evt0
    cmd2 -->|"applied"| evt8
    cmd3 -->|"approved"| evt9
    cmd4 -->|"partial"| evt1
    cmd4 -->|"created"| evt1
    cmd5 -->|"enabled"| evt13
    cmd6 -->|"measured"| evt4
    cmd7 -->|"proposed"| evt7
    cmd7 -->|"reviewed"| evt7
    cmd7 -->|"applied"| evt7
    cmd8 -->|"disabled"| evt12
    cmd8 -->|"counted"| evt5
    cmd9 -->|"rejected"| evt9
    cmd10 -->|"removed"| evt2
    cmd11 -->|"restored"| evt10
    cmd12 -->|"disabled"| evt6
    cmd12 -->|"fetched"| evt14
    cmd12 -->|"ran"| evt15
    cmd13 -->|"updated"| evt3
```

A command is accepted by the component that owns its context, emits the events one of its outcomes declares, and a dashed edge is a binding carrying an event into the next command. Design §9 begins one step earlier, at the actor who invokes the first command, and so does this graph: a solid edge out of an actor is a grant, and an actor drawn with no edge at all may invoke nothing — which is something the model says, not an arrow somebody forgot.

## Bounded contexts

- **[Instances](domains/cortex-instance.md)** (`cortex.instance`) — Instances of a knowledge brain and the data sources that feed them. An instance owns one EKR store, created from the instance spec's seed. A source fetches documents on its own schedule, keeps those that are new or changed, has a model extract them into an EKR extraction document and applies it to the instance's store. 75 types, three entities, two views, 14 commands, 16 events, 30 errors and two actors.

## Components

A component is a unit of ownership, not a deployment. How many of each runs, and what each needs, is [the topology](topology.md).

**`cortex`** — The operator's command line: create, update, run and remove instances, and switch a source off and on. A systemd user timer per source calls the same binary. It owns [`cortex.instance`](domains/cortex-instance.md). It accepts `cortex.instance.AddSource`, `cortex.instance.AdoptInstance`, `cortex.instance.ApplySchemaProposals`, `cortex.instance.ApproveSchemaProposals`, `cortex.instance.CreateInstance`, `cortex.instance.EnableSource`, `cortex.instance.MeasureQuality`, `cortex.instance.ProposeSchemaChanges`, `cortex.instance.RecordFailure`, `cortex.instance.RejectSchemaProposals`, `cortex.instance.RemoveInstance`, `cortex.instance.RestoreSnapshot`, `cortex.instance.RunSource` and `cortex.instance.UpdateInstance`. It publishes `cortex.instance.InstanceAdopted`, `cortex.instance.InstanceCreated`, `cortex.instance.InstanceRemoved`, `cortex.instance.InstanceUpdated`, `cortex.instance.QualityMeasured`, `cortex.instance.RunFailed`, `cortex.instance.SchemaChangesProposed`, `cortex.instance.SchemaProposalsApplied`, `cortex.instance.SchemaProposalsDecided`, `cortex.instance.SnapshotRestored`, `cortex.instance.SourceAdded`, `cortex.instance.SourceDisabled`, `cortex.instance.SourceEnabled` and `cortex.instance.SourceRan`.

## The other pages

| page | what is on it |
|---|---|
| [Instances](domains/cortex-instance.md) | the `cortex.instance` vocabulary: its types, entities, views, commands, events, errors and actors |
| [Interactions](interactions.md) | every binding, with what it guarantees and what happens when it fails |
| [Type crossings](crossings.md) | every conversion this system permits, and the reason someone gave for it |
| [Topology](topology.md) | what each component needs in order to run |


---

Generated from cortex v1 · model digest `590eec96e468842d47bbd36e564dfe80cfcb3f6ff17a1259e23fd89709ba10d7` · contract digest `a29e3a1337a53e4441a5e167da33ade01c37192a6adae9420ea33dbe05308d96`. Do not edit this file; change the specification and regenerate it with `ess generate`.
