<!--
  generated from cortex v1
  model digest 3cb764fa6058c327300de7d63b53b82b3ed86189c61ff97550848ad850d64bcf
  contract digest 5daf559cd61a381fbf6ce9233bce2e7d477c4713ee671105b36029ab93dc414a
  do not edit: regenerate with `ess synthesize --layout crate`
-->
# Synthesis plan — cortex v1

Scope: `component-skeletons`, laid out as `crate`, planned by `ess-synth`. Regenerate with `ess synthesize --layout crate`.

254 capabilities: **246 generated**, **3 obligations**, **5 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `cortex.ask.AnswerId` |
| domain type | `cortex.ask.AnswerOutcome` |
| domain type | `cortex.ask.AnswerRecord.State` |
| domain type | `cortex.ask.AskDelta` |
| domain type | `cortex.ask.AskDiscard` |
| domain type | `cortex.ask.AskDone` |
| domain type | `cortex.ask.AskError` |
| domain type | `cortex.ask.AskEvent` |
| domain type | `cortex.ask.AskLimits` |
| domain type | `cortex.ask.AskSpec` |
| domain type | `cortex.ask.AskStatus` |
| domain type | `cortex.ask.AskTool` |
| domain type | `cortex.ask.Bar` |
| domain type | `cortex.ask.BarChart` |
| domain type | `cortex.ask.FigureSpec` |
| domain type | `cortex.ask.GraphEdge` |
| domain type | `cortex.ask.GraphNode` |
| domain type | `cortex.ask.LinkTemplates` |
| domain type | `cortex.ask.PostgresRecords` |
| domain type | `cortex.ask.RelationshipGraph` |
| domain type | `cortex.ask.ReviewCause` |
| domain type | `cortex.ask.ReviewId` |
| domain type | `cortex.ask.ReviewItem.State` |
| domain type | `cortex.ask.Timeline` |
| domain type | `cortex.ask.TimelineEntry` |
| domain type | `cortex.ask.Vote.State` |
| domain type | `cortex.ask.VoteId` |
| domain type | `cortex.ask.VoteValue` |
| domain type | `cortex.instance.ChangeDetection` |
| domain type | `cortex.instance.ChildCall` |
| domain type | `cortex.instance.CompareLink` |
| domain type | `cortex.instance.ConnectionRef` |
| domain type | `cortex.instance.ConnectorsSource` |
| domain type | `cortex.instance.CrawlPolicy` |
| domain type | `cortex.instance.DailySpend` |
| domain type | `cortex.instance.DocumentId` |
| domain type | `cortex.instance.DropPolicy` |
| domain type | `cortex.instance.EdgeTypeWidening` |
| domain type | `cortex.instance.EkrPin` |
| domain type | `cortex.instance.EvalCase` |
| domain type | `cortex.instance.EvalResult` |
| domain type | `cortex.instance.EvalRun.State` |
| domain type | `cortex.instance.EvalRunner` |
| domain type | `cortex.instance.EvalSet` |
| domain type | `cortex.instance.FetchPolicy` |
| domain type | `cortex.instance.FileRecords` |
| domain type | `cortex.instance.FilesSource` |
| domain type | `cortex.instance.GateCheck` |
| domain type | `cortex.instance.HookResult` |
| domain type | `cortex.instance.Instance.State` |
| domain type | `cortex.instance.InstanceName` |
| domain type | `cortex.instance.InstanceSpec` |
| domain type | `cortex.instance.KnownEntity` |
| domain type | `cortex.instance.KnownOrigin` |
| domain type | `cortex.instance.ModelBackend` |
| domain type | `cortex.instance.ModelGateway` |
| domain type | `cortex.instance.ModelSpec` |
| domain type | `cortex.instance.ModelUnavailableCause` |
| domain type | `cortex.instance.MonitoringSpec` |
| domain type | `cortex.instance.NewEdgeType` |
| domain type | `cortex.instance.NewNodeType` |
| domain type | `cortex.instance.PageStyle` |
| domain type | `cortex.instance.Paging` |
| domain type | `cortex.instance.PostgresStore` |
| domain type | `cortex.instance.PropertyChange` |
| domain type | `cortex.instance.PropertyMapping` |
| domain type | `cortex.instance.PropertyRemoval` |
| domain type | `cortex.instance.ProposedSchemaChange` |
| domain type | `cortex.instance.QualityVerdict` |
| domain type | `cortex.instance.QualityVerdictKind` |
| domain type | `cortex.instance.RecordFilter` |
| domain type | `cortex.instance.RecordFormat` |
| domain type | `cortex.instance.RecordMapping` |
| domain type | `cortex.instance.RedactionClass` |
| domain type | `cortex.instance.RedactionPolicy` |
| domain type | `cortex.instance.RedactionRule` |
| domain type | `cortex.instance.RelationMapping` |
| domain type | `cortex.instance.RunGate` |
| domain type | `cortex.instance.RunHook` |
| domain type | `cortex.instance.RunOutcome` |
| domain type | `cortex.instance.SchemaCardinality` |
| domain type | `cortex.instance.SchemaChange` |
| domain type | `cortex.instance.SchemaProperty` |
| domain type | `cortex.instance.SchemaProposal` |
| domain type | `cortex.instance.SchemaProposalStatus` |
| domain type | `cortex.instance.SchemaProposals` |
| domain type | `cortex.instance.SchemaValueKind` |
| domain type | `cortex.instance.SearchInput` |
| domain type | `cortex.instance.SearchPolicy` |
| domain type | `cortex.instance.SearchTopic` |
| domain type | `cortex.instance.SeedSpec` |
| domain type | `cortex.instance.SeenDocument.State` |
| domain type | `cortex.instance.ServeSpec` |
| domain type | `cortex.instance.SitesInput` |
| domain type | `cortex.instance.SnapshotPolicy` |
| domain type | `cortex.instance.Source.State` |
| domain type | `cortex.instance.SourceId` |
| domain type | `cortex.instance.SourceKind` |
| domain type | `cortex.instance.SourceSettings` |
| domain type | `cortex.instance.SourceSpec` |
| domain type | `cortex.instance.SqliteStore` |
| domain type | `cortex.instance.StoreLookup` |
| domain type | `cortex.instance.StoreSpec` |
| domain type | `cortex.instance.StructuredChild` |
| domain type | `cortex.instance.StructuredConnectors` |
| domain type | `cortex.instance.StructuredFiles` |
| domain type | `cortex.instance.StructuredInput` |
| domain type | `cortex.instance.StructuredSource` |
| domain type | `cortex.instance.TimeRange` |
| domain type | `cortex.instance.TypeMerge` |
| domain type | `cortex.instance.TypeRemoval` |
| domain type | `cortex.instance.TypeSplit` |
| domain type | `cortex.instance.WebInput` |
| domain type | `cortex.instance.WebMode` |
| domain type | `cortex.instance.WebSource` |
| entity lifecycle | `cortex.ask.AnswerRecord` |
| entity lifecycle | `cortex.ask.ReviewItem` |
| entity lifecycle | `cortex.ask.Vote` |
| entity lifecycle | `cortex.instance.EvalRun` |
| entity lifecycle | `cortex.instance.Instance` |
| entity lifecycle | `cortex.instance.SeenDocument` |
| entity lifecycle | `cortex.instance.Source` |
| command contract | `cortex.ask.Ask` |
| command behaviour | `cortex.ask.Ask` |
| command contract | `cortex.ask.CastVote` |
| command behaviour | `cortex.ask.CastVote` |
| command contract | `cortex.ask.CloseReview` |
| command behaviour | `cortex.ask.CloseReview` |
| command contract | `cortex.ask.OpenReview` |
| command behaviour | `cortex.ask.OpenReview` |
| command contract | `cortex.ask.PurgeExpired` |
| command behaviour | `cortex.ask.PurgeExpired` |
| command contract | `cortex.ask.RecordAnswer` |
| command behaviour | `cortex.ask.RecordAnswer` |
| command contract | `cortex.instance.AddEvalCase` |
| command behaviour | `cortex.instance.AddEvalCase` |
| command contract | `cortex.instance.AddSource` |
| command behaviour | `cortex.instance.AddSource` |
| command contract | `cortex.instance.AdoptInstance` |
| command behaviour | `cortex.instance.AdoptInstance` |
| command contract | `cortex.instance.ApplySchemaProposals` |
| command behaviour | `cortex.instance.ApplySchemaProposals` |
| command contract | `cortex.instance.ApproveSchemaProposals` |
| command behaviour | `cortex.instance.ApproveSchemaProposals` |
| command contract | `cortex.instance.CreateInstance` |
| command behaviour | `cortex.instance.CreateInstance` |
| command contract | `cortex.instance.EnableSource` |
| command behaviour | `cortex.instance.EnableSource` |
| command contract | `cortex.instance.MeasureQuality` |
| command behaviour | `cortex.instance.MeasureQuality` |
| command contract | `cortex.instance.ProposeSchemaChanges` |
| command behaviour | `cortex.instance.ProposeSchemaChanges` |
| command contract | `cortex.instance.RecordFailure` |
| command contract | `cortex.instance.RejectSchemaProposals` |
| command behaviour | `cortex.instance.RejectSchemaProposals` |
| command contract | `cortex.instance.RemoveInstance` |
| command behaviour | `cortex.instance.RemoveInstance` |
| command contract | `cortex.instance.RestoreSnapshot` |
| command behaviour | `cortex.instance.RestoreSnapshot` |
| command contract | `cortex.instance.RunEval` |
| command behaviour | `cortex.instance.RunEval` |
| command contract | `cortex.instance.RunSource` |
| command contract | `cortex.instance.UpdateInstance` |
| command behaviour | `cortex.instance.UpdateInstance` |
| event type | `cortex.ask.AnswerRecorded` |
| event type | `cortex.ask.Answered` |
| event type | `cortex.ask.Purged` |
| event type | `cortex.ask.ReviewClosed` |
| event type | `cortex.ask.ReviewOpened` |
| event type | `cortex.ask.Voted` |
| event type | `cortex.instance.EvalCaseAdded` |
| event type | `cortex.instance.EvalRan` |
| event type | `cortex.instance.InstanceAdopted` |
| event type | `cortex.instance.InstanceCreated` |
| event type | `cortex.instance.InstanceRemoved` |
| event type | `cortex.instance.InstanceUpdated` |
| event type | `cortex.instance.QualityMeasured` |
| event type | `cortex.instance.RunFailed` |
| event type | `cortex.instance.RunSkipped` |
| event type | `cortex.instance.SchemaChangesProposed` |
| event type | `cortex.instance.SchemaProposalsApplied` |
| event type | `cortex.instance.SchemaProposalsDecided` |
| event type | `cortex.instance.SnapshotRestored` |
| event type | `cortex.instance.SourceAdded` |
| event type | `cortex.instance.SourceDisabled` |
| event type | `cortex.instance.SourceEnabled` |
| event type | `cortex.instance.SourceFetched` |
| event type | `cortex.instance.SourceRan` |
| error type | `cortex.ask.AnswerNotFound` |
| error type | `cortex.ask.ClientClosed` |
| error type | `cortex.ask.CutOff` |
| error type | `cortex.ask.FigureInvalid` |
| error type | `cortex.ask.FigureTooLarge` |
| error type | `cortex.ask.LimitReached` |
| error type | `cortex.ask.ModelFailed` |
| error type | `cortex.ask.RateLimited` |
| error type | `cortex.ask.ReaderFailed` |
| error type | `cortex.ask.ReviewExists` |
| error type | `cortex.ask.ReviewNotFound` |
| error type | `cortex.ask.ReviewNotOpen` |
| error type | `cortex.ask.Unidentified` |
| error type | `cortex.instance.ApplyRefused` |
| error type | `cortex.instance.BackendMismatch` |
| error type | `cortex.instance.BatchTooLarge` |
| error type | `cortex.instance.ConnectionMissing` |
| error type | `cortex.instance.ConnectionUnavailable` |
| error type | `cortex.instance.EvalCaseExists` |
| error type | `cortex.instance.EvalNoExpectations` |
| error type | `cortex.instance.EvalReviewNotFound` |
| error type | `cortex.instance.EvalReviewOpen` |
| error type | `cortex.instance.EvalRunnerFailed` |
| error type | `cortex.instance.EvalSetInvalid` |
| error type | `cortex.instance.ExtractionFailed` |
| error type | `cortex.instance.FetchFailed` |
| error type | `cortex.instance.InstanceBusy` |
| error type | `cortex.instance.InstanceNotActive` |
| error type | `cortex.instance.InstanceNotFound` |
| error type | `cortex.instance.JudgeFailed` |
| error type | `cortex.instance.ModelUnavailable` |
| error type | `cortex.instance.NameTaken` |
| error type | `cortex.instance.ProposeFailed` |
| error type | `cortex.instance.RestoreUnsupported` |
| error type | `cortex.instance.RunTerminated` |
| error type | `cortex.instance.SampleFailed` |
| error type | `cortex.instance.SchemaApplyFailed` |
| error type | `cortex.instance.SchemaGateFailed` |
| error type | `cortex.instance.SchemaProposalsNotApproved` |
| error type | `cortex.instance.SchemaProposalsNotPending` |
| error type | `cortex.instance.SchemaRunNotFound` |
| error type | `cortex.instance.SeedChangeRefused` |
| error type | `cortex.instance.SeedRefused` |
| error type | `cortex.instance.SeedTypesMissing` |
| error type | `cortex.instance.SnapshotNotFound` |
| error type | `cortex.instance.SourceDisabledError` |
| error type | `cortex.instance.SourceNotDisabled` |
| error type | `cortex.instance.SourceNotFound` |
| error type | `cortex.instance.StoreHeld` |
| error type | `cortex.instance.StoreOutcomeUnknown` |
| error type | `cortex.instance.StoreUnreadable` |
| view type | `cortex.ask.OpenReviews` |
| view type | `cortex.instance.Instances` |
| view query | `cortex.instance.Instances` |
| view type | `cortex.instance.Sources` |
| view query | `cortex.instance.Sources` |
| component port | `cortex` |
| component port | `cortex-ask` |

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
| command behaviour | `cortex.instance.RunSource` | kept an obligation by a refusal that compensates (`compensates: true`), on `fetch-failed` | given `cortex.instance.RunSource` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `disabled` when the existing subject is in Disabled and `(not (defined(fetch_only)) or fetch_only == false)`, updates `cortex.instance.Source`, emits `cortex.instance.RunSkipped`; `fetch-only-failed` externally decided (Connectors or the filesystem fails to deliver the source's documents in a fetch-only run) when eligible ((defined(fetch_only) and fetch_only == true)), error `cortex.instance.FetchFailed`; `fetched` when `(defined(fetch_only) and fetch_only == true)`, updates `cortex.instance.Source`, emits `cortex.instance.SourceFetched`; `fetch-failed` externally decided (Connectors or the filesystem fails to deliver the source's documents; an invocation Connectors refuses with timeout at admission is retried once first), updates `cortex.instance.Source`, error `cortex.instance.FetchFailed`; `extraction-failed` externally decided (The model call fails (other than as `model-unavailable` says), times out, exceeds the run budget before any batch, or returns no valid document; an error answer that cost zero is asked once more first, after a fixed wait), updates `cortex.instance.Source`, error `cortex.instance.ExtractionFailed`; `apply-refused` externally decided (EKR refuses the merged extraction document), updates `cortex.instance.Source`, error `cortex.instance.ApplyRefused`; `store-outcome-unknown` externally decided (EKR answers that the publication outcome of a store write of the run (a stage verb or `apply-extraction`) is unknown), updates `cortex.instance.Source`, error `cortex.instance.StoreOutcomeUnknown`; `connection-unavailable` externally decided (Connectors answers a retryable operational failure to every attempt of `connections revalidate` (or of the `connections list` read before it, which for a source naming a profile is also the read that resolves the profile) before the run's first fetch), updates `cortex.instance.Source`, error `cortex.instance.ConnectionUnavailable`; `batch-too-large` externally decided (`claude` answers that a part of several documents of an extraction batch is too long for the model's window, or that its answer was cut off at the output limit, after the batch was split 3 deep), updates `cortex.instance.Source`, error `cortex.instance.BatchTooLarge`; `terminated` externally decided (The run is sent SIGTERM before it ends), updates `cortex.instance.Source`, error `cortex.instance.RunTerminated`; `model-unavailable` externally decided (The run's model cannot be reached: its process cannot be started, its connection is refused, the server behind the gateway has not loaded the model, or the gateway answers HTTP 502, 503, 504 or 529 (overloaded)), updates `cortex.instance.Source`, error `cortex.instance.ModelUnavailable`; `ran` otherwise, updates `cortex.instance.Source`, emits `cortex.instance.SourceRan`; `no-such-source` for an identity no record carries, error `cortex.instance.SourceNotFound` |
| view query | `cortex.ask.OpenReviews` | kept an obligation by an order over the `Timestamp` field `opened_at`, which the suite ranks by its instant and a generated query does not | a query answering `cortex.ask.OpenReviews` with rows projected from `cortex.ask.ReviewItem` at `read_your_writes` consistency, containing instances where `state == Open` |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
| actor grants | `cortex.ask.Asker` | planning | may invoke `cortex.ask.Ask`, `cortex.ask.CastVote`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `cortex.ask.Reviewer` | planning | may invoke `cortex.ask.CloseReview`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `cortex.ask.Service` | planning | may invoke `cortex.ask.OpenReview`, `cortex.ask.PurgeExpired`, `cortex.ask.RecordAnswer`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `cortex.instance.Operator` | planning | may invoke `cortex.instance.AddEvalCase`, `cortex.instance.AddSource`, `cortex.instance.AdoptInstance`, `cortex.instance.ApplySchemaProposals`, `cortex.instance.ApproveSchemaProposals`, `cortex.instance.CreateInstance`, `cortex.instance.EnableSource`, `cortex.instance.MeasureQuality`, `cortex.instance.ProposeSchemaChanges`, `cortex.instance.RejectSchemaProposals`, `cortex.instance.RemoveInstance`, `cortex.instance.RestoreSnapshot`, `cortex.instance.RunEval`, `cortex.instance.UpdateInstance`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
| actor grants | `cortex.instance.Scheduler` | planning | may invoke `cortex.instance.RecordFailure`, `cortex.instance.RunSource`; generated as data, not enforced: the grant is available as the declared actors and the qualified commands each may invoke, and enforcement stays with the caller, because a grant is checked against a caller identity, which types do not carry |
