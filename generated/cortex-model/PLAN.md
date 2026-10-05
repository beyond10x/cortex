<!--
  generated from cortex v1
  model digest 7b83a57875607ec97b9c579958bfec9caf298b92a55106e478780b1216e04f43
  contract digest 28f44d88be3d554adbf5e4d04488470231c93d64c08ef39e288ec9c2d93f34e4
  do not edit: regenerate with `ess synthesize --layout crate`
-->
# Synthesis plan — cortex v1

Scope: `component-skeletons`, laid out as `crate`, planned by `ess-synth`. Regenerate with `ess synthesize --layout crate`.

68 capabilities: **64 generated**, **2 obligations**, **2 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `cortex.instance.ChangeDetection` |
| domain type | `cortex.instance.ConnectorsSource` |
| domain type | `cortex.instance.CrawlPolicy` |
| domain type | `cortex.instance.EkrPin` |
| domain type | `cortex.instance.FetchPolicy` |
| domain type | `cortex.instance.FilesSource` |
| domain type | `cortex.instance.Instance.State` |
| domain type | `cortex.instance.InstanceName` |
| domain type | `cortex.instance.InstanceSpec` |
| domain type | `cortex.instance.ModelSpec` |
| domain type | `cortex.instance.SearchInput` |
| domain type | `cortex.instance.SearchPolicy` |
| domain type | `cortex.instance.SearchTopic` |
| domain type | `cortex.instance.SeedSpec` |
| domain type | `cortex.instance.ServeSpec` |
| domain type | `cortex.instance.SitesInput` |
| domain type | `cortex.instance.Source.State` |
| domain type | `cortex.instance.SourceId` |
| domain type | `cortex.instance.SourceKind` |
| domain type | `cortex.instance.SourceSettings` |
| domain type | `cortex.instance.SourceSpec` |
| domain type | `cortex.instance.TimeRange` |
| domain type | `cortex.instance.WebInput` |
| domain type | `cortex.instance.WebMode` |
| domain type | `cortex.instance.WebSource` |
| entity lifecycle | `cortex.instance.Instance` |
| entity lifecycle | `cortex.instance.Source` |
| command contract | `cortex.instance.AddSource` |
| command behaviour | `cortex.instance.AddSource` |
| command contract | `cortex.instance.CreateInstance` |
| command behaviour | `cortex.instance.CreateInstance` |
| command contract | `cortex.instance.EnableSource` |
| command behaviour | `cortex.instance.EnableSource` |
| command contract | `cortex.instance.RecordFailure` |
| command contract | `cortex.instance.RemoveInstance` |
| command behaviour | `cortex.instance.RemoveInstance` |
| command contract | `cortex.instance.RunSource` |
| command contract | `cortex.instance.UpdateInstance` |
| command behaviour | `cortex.instance.UpdateInstance` |
| event type | `cortex.instance.InstanceCreated` |
| event type | `cortex.instance.InstanceRemoved` |
| event type | `cortex.instance.InstanceUpdated` |
| event type | `cortex.instance.RunFailed` |
| event type | `cortex.instance.SourceAdded` |
| event type | `cortex.instance.SourceDisabled` |
| event type | `cortex.instance.SourceEnabled` |
| event type | `cortex.instance.SourceRan` |
| error type | `cortex.instance.ApplyRefused` |
| error type | `cortex.instance.ConnectionMissing` |
| error type | `cortex.instance.ExtractionFailed` |
| error type | `cortex.instance.FetchFailed` |
| error type | `cortex.instance.InstanceNotActive` |
| error type | `cortex.instance.InstanceNotFound` |
| error type | `cortex.instance.NameTaken` |
| error type | `cortex.instance.SeedChangeRefused` |
| error type | `cortex.instance.SeedRefused` |
| error type | `cortex.instance.SourceDisabledError` |
| error type | `cortex.instance.SourceNotDisabled` |
| error type | `cortex.instance.SourceNotFound` |
| view type | `cortex.instance.Instances` |
| view query | `cortex.instance.Instances` |
| view type | `cortex.instance.Sources` |
| view query | `cortex.instance.Sources` |
| component port | `cortex` |

## Ports — yours to provide

What the specification fully determines is generated; what it cannot determine is an obligation. A generated command behaviour or view query reads and writes through the ports below, and they are yours to provide: synthesis generates each port's contract and never an implementation of one, so where instances live stays your decision.

| port | what it answers |
| --- | --- |
| storage | one per entity a generated behaviour or query reads or writes: the instance stored under an identity; storing, replacing and removing one; and every stored instance, in the order the store keeps them |
| context | where a generated behaviour asks it: the caller's attributes, every identity and value the specification says the implementation assigns, and whether each `external:` branch is taken |

## Obligations — yours to implement

| capability | source | why not generated | contract |
| --- | --- | --- | --- |
| command behaviour | `cortex.instance.RecordFailure` | kept an obligation by a subject predicate choosing between a move and an update | given `cortex.instance.RecordFailure` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `disabled` when the existing subject's stored fields satisfy `consecutive_failures >= 1`, takes `disable` of `cortex.instance.Source`, emits `cortex.instance.SourceDisabled`; `counted` otherwise, updates `cortex.instance.Source`, emits `cortex.instance.RunFailed`; `already-disabled` from a state no declared move starts in, error `cortex.instance.SourceDisabledError`; `no-such-source` for an identity no record carries, error `cortex.instance.SourceNotFound` |
| command behaviour | `cortex.instance.RunSource` | kept an obligation by `when_subject_state:` beside `external:` in one command | given `cortex.instance.RunSource` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `fetch-failed` externally decided (Connectors or the filesystem fails to deliver the source's documents), error `cortex.instance.FetchFailed`; `extraction-failed` externally decided (The model call fails, times out, exceeds the run budget before any batch, or returns no valid document), error `cortex.instance.ExtractionFailed`; `apply-refused` externally decided (EKR refuses the merged extraction document), error `cortex.instance.ApplyRefused`; `ran` when the existing subject is in Enabled, updates `cortex.instance.Source`, emits `cortex.instance.SourceRan`; `disabled` otherwise, error `cortex.instance.SourceDisabledError`; `no-such-source` for an identity no record carries, error `cortex.instance.SourceNotFound` |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
| actor grants | `cortex.instance.Operator` | planning | may invoke `cortex.instance.AddSource`, `cortex.instance.CreateInstance`, `cortex.instance.EnableSource`, `cortex.instance.RemoveInstance`, `cortex.instance.UpdateInstance`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `cortex.instance.Scheduler` | planning | may invoke `cortex.instance.RecordFailure`, `cortex.instance.RunSource`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
