// generated from cortex v1
// model digest 33b99b070654ace12840af9a0ffd38b5565691f60a37e5e0795d7c799d5ad121
// contract digest f931c0a4605acc8bda17aba87eb5f8f31cdf6c1d7e01f81e01dca43dfdb8c3a4
// do not edit: regenerate with `ess synthesize --layout crate`

//! Instances — `cortex.instance`.
//!
//! Instances of a knowledge brain and the data sources that feed them. An instance owns one EKR store, created from the instance spec's seed. A source fetches documents on its own schedule, keeps those that are new or changed, has a model extract them into an EKR extraction document and applies it to the instance's store.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// ChangeDetection — `cortex.instance.ChangeDetection`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeDetection {
    /// `ContentHash`.
    ContentHash,
}

/// ChildCall — `cortex.instance.ChildCall`.
///
/// Every value satisfies `paging.stop_older_than_since != true` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildCall {
    /// `operation` — `String`.
    pub operation: String,
    /// `input` — `Json`.
    pub input: crate::json::Value,
    /// `records` — `String`.
    pub records: String,
    /// `paging` — `Optional<cortex.instance.Paging>`.
    pub paging: Option<Paging>,
}

/// CompareLink — `cortex.instance.CompareLink`.
///
/// Every value satisfies `paging.stop_older_than_since != true` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareLink {
    /// `operation` — `String`.
    pub operation: String,
    /// `input` — `Json`.
    pub input: crate::json::Value,
    /// `records` — `String`.
    pub records: String,
    /// `paging` — `Optional<cortex.instance.Paging>`.
    pub paging: Option<Paging>,
    /// `tags` — `String`.
    pub tags: String,
    /// `changes` — `String`.
    pub changes: String,
    /// `order` — `String`.
    pub order: String,
    /// `change_id` — `String`.
    pub change_id: String,
    /// `relation` — `String`.
    pub relation: String,
}

/// ConnectionRef — `cortex.instance.ConnectionRef`.
///
/// Every value satisfies `(defined(connection) or defined(profile))` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionRef {
    /// `adapter` — `String`.
    pub adapter: String,
    /// `connection` — `Optional<String>`.
    pub connection: Option<String>,
    /// `profile` — `Optional<String>`.
    pub profile: Option<String>,
}

/// ConnectorsSource — `cortex.instance.ConnectorsSource`.
///
/// Every value satisfies `(paging.stop_older_than_since != true or defined(time))` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `(defined(connection) or defined(profile))` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorsSource {
    /// `adapter` — `String`.
    pub adapter: String,
    /// `connection` — `Optional<String>`.
    pub connection: Option<String>,
    /// `profile` — `Optional<String>`.
    pub profile: Option<String>,
    /// `operation` — `String`.
    pub operation: String,
    /// `inputs` — `List<Json>`.
    pub inputs: Vec<crate::json::Value>,
    /// `records` — `String`.
    pub records: String,
    /// `id` — `String`.
    pub id: String,
    /// `time` — `Optional<String>`.
    pub time: Option<String>,
    /// `text` — `List<String>`.
    pub text: Vec<String>,
    /// `paging` — `Optional<cortex.instance.Paging>`.
    pub paging: Option<Paging>,
    /// `child` — `Optional<cortex.instance.ChildCall>`.
    pub child: Option<ChildCall>,
}

/// CrawlPolicy — `cortex.instance.CrawlPolicy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlPolicy {
    /// `max_depth` — `Integer`.
    pub max_depth: i64,
    /// `max_breadth` — `Integer`.
    pub max_breadth: i64,
    /// `limit` — `Integer`.
    pub limit: i64,
    /// `select_paths` — `List<String>`.
    pub select_paths: Vec<String>,
    /// `exclude_paths` — `List<String>`.
    pub exclude_paths: Vec<String>,
    /// `instructions` — `Optional<String>`.
    pub instructions: Option<String>,
    /// `allow_external` — `Boolean`.
    pub allow_external: bool,
}

/// DailySpend — `cortex.instance.DailySpend`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailySpend {
    /// `format` — `String`.
    pub format: String,
    /// `day` — `String`.
    pub day: String,
    /// `spent_usd` — `Decimal`.
    pub spent_usd: crate::primitives::Decimal,
    /// `spent_tokens` — `Optional<Integer>`.
    pub spent_tokens: Option<i64>,
    /// `uncounted_usd` — `Optional<Decimal>`.
    pub uncounted_usd: Option<crate::primitives::Decimal>,
}

/// DocumentId — `cortex.instance.DocumentId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentId(pub String);

/// DropPolicy — `cortex.instance.DropPolicy`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPolicy {
    /// `Keep`.
    Keep,
    /// `Supersede`.
    Supersede,
}

/// EdgeTypeWidening — `cortex.instance.EdgeTypeWidening`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTypeWidening {
    /// `name` — `String`.
    pub name: String,
    /// `add_source_types` — `List<String>`.
    pub add_source_types: Vec<String>,
    /// `add_target_types` — `List<String>`.
    pub add_target_types: Vec<String>,
}

/// EkrPin — `cortex.instance.EkrPin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EkrPin {
    /// `version` — `String`.
    pub version: String,
    /// `bin` — `Optional<String>`.
    pub bin: Option<String>,
}

/// EvalCase — `cortex.instance.EvalCase`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCase {
    /// `format` — `String`.
    pub format: String,
    /// `id` — `String`.
    pub id: String,
    /// `question` — `String`.
    pub question: String,
    /// `expect_facts` — `List<String>`.
    pub expect_facts: Vec<String>,
    /// `expect_nodes` — `List<String>`.
    pub expect_nodes: Vec<String>,
    /// `source` — `Optional<String>`.
    pub source: Option<String>,
    /// `review_id` — `Optional<cortex.ask.ReviewId>`.
    pub review_id: Option<crate::ask::ReviewId>,
}

/// EvalResult — `cortex.instance.EvalResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalResult {
    /// `format` — `String`.
    pub format: String,
    /// `id` — `String`.
    pub id: String,
    /// `question` — `String`.
    pub question: String,
    /// `answer` — `String`.
    pub answer: String,
    /// `cited_nodes` — `List<String>`.
    pub cited_nodes: Vec<String>,
    /// `passed` — `Boolean`.
    pub passed: bool,
    /// `missing_facts` — `List<String>`.
    pub missing_facts: Vec<String>,
    /// `missing_nodes` — `List<String>`.
    pub missing_nodes: Vec<String>,
}

/// The states of `cortex.instance.EvalRun`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `EvalRun<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalRunState {
    /// `Recorded`.
    Recorded,
}

/// EvalRunner — `cortex.instance.EvalRunner`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalRunner {
    /// `mcp`.
    Mcp,
    /// `ask`.
    Ask,
    /// `replay`.
    Replay,
}

/// EvalSet — `cortex.instance.EvalSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalSet {
    /// `cases` — `List<cortex.instance.EvalCase>`.
    pub cases: Vec<EvalCase>,
}

/// FetchPolicy — `cortex.instance.FetchPolicy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchPolicy {
    /// `refresh_after_days` — `Integer`.
    pub refresh_after_days: i64,
    /// `change` — `cortex.instance.ChangeDetection`.
    pub change: ChangeDetection,
    /// `max_documents_per_run` — `Integer`.
    pub max_documents_per_run: i64,
    /// `max_chars_per_document` — `Integer`.
    pub max_chars_per_document: i64,
}

/// FileRecords — `cortex.instance.FileRecords`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRecords {
    /// `format` — `cortex.instance.RecordFormat`.
    pub format: RecordFormat,
    /// `id` — `String`.
    pub id: String,
    /// `time` — `Optional<String>`.
    pub time: Option<String>,
    /// `author` — `Optional<String>`.
    pub author: Option<String>,
    /// `text` — `List<String>`.
    pub text: Vec<String>,
    /// `fallback_text` — `Optional<List<String>>`.
    pub fallback_text: Option<Vec<String>>,
    /// `thread` — `Optional<String>`.
    pub thread: Option<String>,
    /// `filters` — `List<cortex.instance.RecordFilter>`.
    pub filters: Vec<RecordFilter>,
    /// `lookup` — `Optional<String>`.
    pub lookup: Option<String>,
}

/// FilesSource — `cortex.instance.FilesSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesSource {
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
    /// `glob` — `String`.
    pub glob: String,
    /// `records` — `Optional<cortex.instance.FileRecords>`.
    pub records: Option<FileRecords>,
}

/// GateCheck — `cortex.instance.GateCheck`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCheck {
    /// `measure` — `String`.
    pub measure: String,
    /// `min` — `Optional<Decimal>`.
    pub min: Option<crate::primitives::Decimal>,
    /// `max` — `Optional<Decimal>`.
    pub max: Option<crate::primitives::Decimal>,
}

/// HookResult — `cortex.instance.HookResult`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    /// `called`.
    Called,
    /// `failed`.
    Failed,
    /// `timed-out`.
    TimedOut,
}

/// The states of `cortex.instance.Instance`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Instance<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    /// `Active`.
    Active,
    /// `Removed`.
    Removed,
}

/// InstanceName — `cortex.instance.InstanceName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceName(pub String);

/// InstanceSpec — `cortex.instance.InstanceSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceSpec {
    /// `format` — `String`.
    pub format: String,
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `description` — `String`.
    pub description: String,
    /// `ekr` — `cortex.instance.EkrPin`.
    pub ekr: EkrPin,
    /// `seed` — `cortex.instance.SeedSpec`.
    pub seed: SeedSpec,
    /// `model` — `cortex.instance.ModelSpec`.
    pub model: ModelSpec,
    /// `sources` — `List<cortex.instance.SourceSpec>`.
    pub sources: Vec<SourceSpec>,
    /// `serve` — `cortex.instance.ServeSpec`.
    pub serve: ServeSpec,
    /// `store` — `Optional<cortex.instance.StoreSpec>`.
    pub store: Option<StoreSpec>,
    /// `redaction` — `Optional<cortex.instance.RedactionPolicy>`.
    pub redaction: Option<RedactionPolicy>,
    /// `snapshots` — `Optional<cortex.instance.SnapshotPolicy>`.
    pub snapshots: Option<SnapshotPolicy>,
    /// `gate` — `Optional<cortex.instance.RunGate>`.
    pub gate: Option<RunGate>,
    /// `monitoring` — `Optional<cortex.instance.MonitoringSpec>`.
    pub monitoring: Option<MonitoringSpec>,
    /// `ask` — `Optional<cortex.ask.AskSpec>`.
    pub ask: Option<crate::ask::AskSpec>,
}

/// KnownEntity — `cortex.instance.KnownEntity`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownEntity {
    /// `node_type` — `String`.
    pub node_type: String,
    /// `name` — `String`.
    pub name: String,
    /// `aliases` — `List<String>`.
    pub aliases: Vec<String>,
    /// `origin` — `cortex.instance.KnownOrigin`.
    pub origin: KnownOrigin,
}

/// KnownOrigin — `cortex.instance.KnownOrigin`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownOrigin {
    /// `Store`.
    Store,
    /// `Run`.
    Run,
}

/// ModelBackend — `cortex.instance.ModelBackend`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelBackend {
    /// `Claude`.
    Claude,
    /// `Codex`.
    Codex,
}

/// ModelGateway — `cortex.instance.ModelGateway`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelGateway {
    /// `base_url` — `String`.
    pub base_url: String,
    /// `token_env` — `String`.
    pub token_env: String,
    /// `max_context_tokens` — `Optional<Integer>`.
    pub max_context_tokens: Option<i64>,
    /// `max_output_tokens` — `Optional<Integer>`.
    pub max_output_tokens: Option<i64>,
}

/// ModelSpec — `cortex.instance.ModelSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpec {
    /// `model` — `String`.
    pub model: String,
    /// `backend` — `Optional<cortex.instance.ModelBackend>`.
    pub backend: Option<ModelBackend>,
    /// `budget_usd` — `Optional<Decimal>`.
    pub budget_usd: Option<crate::primitives::Decimal>,
    /// `daily_ceiling_usd` — `Optional<Decimal>`.
    pub daily_ceiling_usd: Option<crate::primitives::Decimal>,
    /// `budget_tokens` — `Optional<Integer>`.
    pub budget_tokens: Option<i64>,
    /// `daily_ceiling_tokens` — `Optional<Integer>`.
    pub daily_ceiling_tokens: Option<i64>,
    /// `gateway` — `Optional<cortex.instance.ModelGateway>`.
    pub gateway: Option<ModelGateway>,
    /// `timeout_s` — `Integer`.
    pub timeout_s: i64,
    /// `instructions` — `Optional<String>`.
    pub instructions: Option<String>,
    /// `batch_chars` — `Optional<Integer>`.
    pub batch_chars: Option<i64>,
}

/// MonitoringSpec — `cortex.instance.MonitoringSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitoringSpec {
    /// `metrics` — `Optional<Boolean>`.
    pub metrics: Option<bool>,
    /// `hook` — `Optional<cortex.instance.RunHook>`.
    pub hook: Option<RunHook>,
}

/// NewEdgeType — `cortex.instance.NewEdgeType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEdgeType {
    /// `name` — `String`.
    pub name: String,
    /// `source_types` — `List<String>`.
    pub source_types: Vec<String>,
    /// `target_types` — `List<String>`.
    pub target_types: Vec<String>,
}

/// NewNodeType — `cortex.instance.NewNodeType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNodeType {
    /// `name` — `String`.
    pub name: String,
    /// `properties` — `List<cortex.instance.SchemaProperty>`.
    pub properties: Vec<SchemaProperty>,
}

/// PageStyle — `cortex.instance.PageStyle`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageStyle {
    /// `PageNumber`.
    PageNumber,
    /// `Token`.
    Token,
    /// `Keyset`.
    Keyset,
    /// `NextLink`.
    NextLink,
}

/// Paging — `cortex.instance.Paging`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paging {
    /// `style` — `cortex.instance.PageStyle`.
    pub style: PageStyle,
    /// `param` — `String`.
    pub param: String,
    /// `next` — `Optional<String>`.
    pub next: Option<String>,
    /// `max_pages` — `Integer`.
    pub max_pages: i64,
    /// `stop_older_than_since` — `Optional<Boolean>`.
    pub stop_older_than_since: Option<bool>,
}

/// PostgresStore — `cortex.instance.PostgresStore`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresStore {
    /// `config` — `String`.
    pub config: String,
    /// `connection` — `Optional<cortex.instance.ConnectionRef>`.
    pub connection: Option<ConnectionRef>,
    /// `schema_connection` — `Optional<cortex.instance.ConnectionRef>`.
    pub schema_connection: Option<ConnectionRef>,
}

/// PropertyChange — `cortex.instance.PropertyChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyChange {
    /// `owner` — `String`.
    pub owner: String,
    /// `property` — `cortex.instance.SchemaProperty`.
    pub property: SchemaProperty,
}

/// PropertyMapping — `cortex.instance.PropertyMapping`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyMapping {
    /// `property` — `String`.
    pub property: String,
    /// `path` — `String`.
    pub path: String,
    /// `value_kind` — `Optional<cortex.instance.SchemaValueKind>`.
    pub value_kind: Option<SchemaValueKind>,
}

/// PropertyRemoval — `cortex.instance.PropertyRemoval`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRemoval {
    /// `owner` — `String`.
    pub owner: String,
    /// `property` — `String`.
    pub property: String,
}

/// ProposedSchemaChange — `cortex.instance.ProposedSchemaChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedSchemaChange {
    /// `change` — `cortex.instance.SchemaChange`.
    pub change: SchemaChange,
    /// `reason` — `String`.
    pub reason: String,
    /// `facts` — `List<String>`.
    pub facts: Vec<String>,
    /// `evidence` — `List<String>`.
    pub evidence: Vec<String>,
}

/// QualityVerdict — `cortex.instance.QualityVerdict`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityVerdict {
    /// `format` — `String`.
    pub format: String,
    /// `fact` — `String`.
    pub fact: String,
    /// `verdict` — `cortex.instance.QualityVerdictKind`.
    pub verdict: QualityVerdictKind,
    /// `reason` — `String`.
    pub reason: String,
}

/// QualityVerdictKind — `cortex.instance.QualityVerdictKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityVerdictKind {
    /// `yes`.
    Yes,
    /// `no`.
    No,
    /// `unclear`.
    Unclear,
}

/// RecordFilter — `cortex.instance.RecordFilter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFilter {
    /// `field` — `String`.
    pub field: String,
    /// `values` — `List<String>`.
    pub values: Vec<String>,
    /// `include` — `Boolean`.
    pub include: bool,
}

/// RecordFormat — `cortex.instance.RecordFormat`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordFormat {
    /// `WholeFile`.
    WholeFile,
    /// `JsonLines`.
    JsonLines,
    /// `MarkdownSections`.
    MarkdownSections,
}

/// RecordMapping — `cortex.instance.RecordMapping`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordMapping {
    /// `node_type` — `String`.
    pub node_type: String,
    /// `id` — `String`.
    pub id: String,
    /// `name` — `String`.
    pub name: String,
    /// `aliases` — `List<String>`.
    pub aliases: Vec<String>,
    /// `properties` — `List<cortex.instance.PropertyMapping>`.
    pub properties: Vec<PropertyMapping>,
    /// `relations` — `List<cortex.instance.RelationMapping>`.
    pub relations: Vec<RelationMapping>,
    /// `alias_templates` — `Optional<List<String>>`.
    pub alias_templates: Option<Vec<String>>,
}

/// RedactionClass — `cortex.instance.RedactionClass`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionClass {
    /// `Email`.
    Email,
    /// `Phone`.
    Phone,
    /// `IpAddress`.
    IpAddress,
    /// `PaymentCard`.
    PaymentCard,
    /// `Url`.
    Url,
    /// `Credential`.
    Credential,
    /// `RareName`.
    RareName,
}

/// RedactionPolicy — `cortex.instance.RedactionPolicy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionPolicy {
    /// `classes` — `List<cortex.instance.RedactionClass>`.
    pub classes: Vec<RedactionClass>,
    /// `rules` — `List<cortex.instance.RedactionRule>`.
    pub rules: Vec<RedactionRule>,
    /// `known_names` — `Optional<List<String>>`.
    pub known_names: Option<Vec<String>>,
    /// `rare_limit` — `Optional<Integer>`.
    pub rare_limit: Option<i64>,
    /// `refuse_if_left` — `Optional<List<cortex.instance.RedactionClass>>`.
    pub refuse_if_left: Option<Vec<RedactionClass>>,
}

/// RedactionRule — `cortex.instance.RedactionRule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionRule {
    /// `name` — `String`.
    pub name: String,
    /// `pattern` — `String`.
    pub pattern: String,
    /// `replacement` — `String`.
    pub replacement: String,
}

/// RelationMapping — `cortex.instance.RelationMapping`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationMapping {
    /// `relation` — `String`.
    pub relation: String,
    /// `target_type` — `String`.
    pub target_type: String,
    /// `target_name` — `String`.
    pub target_name: String,
}

/// RunGate — `cortex.instance.RunGate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunGate {
    /// `checks` — `List<cortex.instance.GateCheck>`.
    pub checks: Vec<GateCheck>,
}

/// RunHook — `cortex.instance.RunHook`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunHook {
    /// `command` — `List<String>`.
    pub command: Vec<String>,
    /// `timeout_s` — `Optional<Integer>`.
    pub timeout_s: Option<i64>,
}

/// RunOutcome — `cortex.instance.RunOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    /// `ran`.
    Ran,
    /// `fetch-failed`.
    FetchFailed,
    /// `extraction-failed`.
    ExtractionFailed,
    /// `apply-refused`.
    ApplyRefused,
    /// `store-outcome-unknown`.
    StoreOutcomeUnknown,
    /// `connection-unavailable`.
    ConnectionUnavailable,
    /// `batch-too-large`.
    BatchTooLarge,
    /// `disabled`.
    Disabled,
}

/// SchemaCardinality — `cortex.instance.SchemaCardinality`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaCardinality {
    /// `One`.
    One,
    /// `Many`.
    Many,
}

/// SchemaChange — `cortex.instance.SchemaChange`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaChange {
    /// Tagged `add_edge_type` — `cortex.instance.NewEdgeType`.
    AddEdgeType(NewEdgeType),
    /// Tagged `add_node_type` — `cortex.instance.NewNodeType`.
    AddNodeType(NewNodeType),
    /// Tagged `add_property` — `cortex.instance.PropertyChange`.
    AddProperty(PropertyChange),
    /// Tagged `merge_types` — `cortex.instance.TypeMerge`.
    MergeTypes(TypeMerge),
    /// Tagged `redeclare_property` — `cortex.instance.PropertyChange`.
    RedeclareProperty(PropertyChange),
    /// Tagged `remove_edge_type` — `cortex.instance.TypeRemoval`.
    RemoveEdgeType(TypeRemoval),
    /// Tagged `remove_node_type` — `cortex.instance.TypeRemoval`.
    RemoveNodeType(TypeRemoval),
    /// Tagged `remove_property` — `cortex.instance.PropertyRemoval`.
    RemoveProperty(PropertyRemoval),
    /// Tagged `split_type` — `cortex.instance.TypeSplit`.
    SplitType(TypeSplit),
    /// Tagged `widen_edge_type` — `cortex.instance.EdgeTypeWidening`.
    WidenEdgeType(EdgeTypeWidening),
}

/// SchemaProperty — `cortex.instance.SchemaProperty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProperty {
    /// `name` — `String`.
    pub name: String,
    /// `value_kind` — `cortex.instance.SchemaValueKind`.
    pub value_kind: SchemaValueKind,
    /// `cardinality` — `cortex.instance.SchemaCardinality`.
    pub cardinality: SchemaCardinality,
}

/// SchemaProposal — `cortex.instance.SchemaProposal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposal {
    /// `format` — `String`.
    pub format: String,
    /// `change` — `cortex.instance.SchemaChange`.
    pub change: SchemaChange,
    /// `reason` — `String`.
    pub reason: String,
    /// `facts` — `List<String>`.
    pub facts: Vec<String>,
    /// `evidence` — `List<String>`.
    pub evidence: Vec<String>,
    /// `status` — `cortex.instance.SchemaProposalStatus`.
    pub status: SchemaProposalStatus,
    /// `codes` — `List<String>`.
    pub codes: Vec<String>,
    /// `note` — `Optional<String>`.
    pub note: Option<String>,
    /// `revision` — `Optional<Integer>`.
    pub revision: Option<i64>,
    /// `schema_version` — `Optional<String>`.
    pub schema_version: Option<String>,
    /// `decision_reason` — `Optional<String>`.
    pub decision_reason: Option<String>,
    /// `decided_by` — `Optional<String>`.
    pub decided_by: Option<String>,
}

/// SchemaProposalStatus — `cortex.instance.SchemaProposalStatus`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaProposalStatus {
    /// `pending`.
    Pending,
    /// `approved`.
    Approved,
    /// `rejected`.
    Rejected,
    /// `applied`.
    Applied,
    /// `refused`.
    Refused,
    /// `recorded-only`.
    RecordedOnly,
    /// `invalid`.
    Invalid,
    /// `dry-run`.
    DryRun,
}

/// SchemaProposals — `cortex.instance.SchemaProposals`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposals {
    /// `proposals` — `List<cortex.instance.ProposedSchemaChange>`.
    pub proposals: Vec<ProposedSchemaChange>,
}

/// SchemaValueKind — `cortex.instance.SchemaValueKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaValueKind {
    /// `String`.
    String,
    /// `Integer`.
    Integer,
    /// `Decimal`.
    Decimal,
    /// `Boolean`.
    Boolean,
    /// `Timestamp`.
    Timestamp,
}

/// SearchInput — `cortex.instance.SearchInput`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchInput {
    /// `queries` — `List<String>`.
    pub queries: Vec<String>,
    /// `policy` — `cortex.instance.SearchPolicy`.
    pub policy: SearchPolicy,
}

/// SearchPolicy — `cortex.instance.SearchPolicy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPolicy {
    /// `topic` — `cortex.instance.SearchTopic`.
    pub topic: SearchTopic,
    /// `time_range` — `Optional<cortex.instance.TimeRange>`.
    pub time_range: Option<TimeRange>,
    /// `max_results` — `Integer`.
    pub max_results: i64,
    /// `include_domains` — `List<String>`.
    pub include_domains: Vec<String>,
    /// `exclude_domains` — `List<String>`.
    pub exclude_domains: Vec<String>,
    /// `country` — `Optional<String>`.
    pub country: Option<String>,
    /// `language` — `Optional<String>`.
    pub language: Option<String>,
}

/// SearchTopic — `cortex.instance.SearchTopic`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTopic {
    /// `general`.
    General,
    /// `news`.
    News,
}

/// SeedSpec — `cortex.instance.SeedSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedSpec {
    /// `schema` — `Optional<String>`.
    pub schema: Option<String>,
    /// `ekr_seed` — `Optional<String>`.
    pub ekr_seed: Option<String>,
    /// `documents` — `List<String>`.
    pub documents: Vec<String>,
}

/// The states of `cortex.instance.SeenDocument`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `SeenDocument<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeenDocumentState {
    /// `Seen`.
    Seen,
}

/// ServeSpec — `cortex.instance.ServeSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServeSpec {
    /// `view_port` — `Optional<Integer>`.
    pub view_port: Option<i64>,
}

/// SitesInput — `cortex.instance.SitesInput`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitesInput {
    /// `urls` — `List<String>`.
    pub urls: Vec<String>,
    /// `mode` — `cortex.instance.WebMode`.
    pub mode: WebMode,
    /// `policy` — `Optional<cortex.instance.CrawlPolicy>`.
    pub policy: Option<CrawlPolicy>,
}

/// SnapshotPolicy — `cortex.instance.SnapshotPolicy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPolicy {
    /// `keep` — `Integer`.
    pub keep: i64,
}

/// The states of `cortex.instance.Source`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Source<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceState {
    /// `Disabled`.
    Disabled,
    /// `Enabled`.
    Enabled,
}

/// SourceId — `cortex.instance.SourceId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceId(pub String);

/// SourceKind — `cortex.instance.SourceKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// `Web`.
    Web,
    /// `Connectors`.
    Connectors,
    /// `Files`.
    Files,
    /// `Structured`.
    Structured,
}

/// SourceSettings — `cortex.instance.SourceSettings`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSettings {
    /// Tagged `connectors` — `cortex.instance.ConnectorsSource`.
    Connectors(ConnectorsSource),
    /// Tagged `files` — `cortex.instance.FilesSource`.
    Files(FilesSource),
    /// Tagged `structured` — `cortex.instance.StructuredSource`.
    Structured(StructuredSource),
    /// Tagged `web` — `cortex.instance.WebSource`.
    Web(WebSource),
}

/// SourceSpec — `cortex.instance.SourceSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpec {
    /// `name` — `String`.
    pub name: String,
    /// `schedule` — `String`.
    pub schedule: String,
    /// `settings` — `cortex.instance.SourceSettings`.
    pub settings: SourceSettings,
    /// `policy` — `cortex.instance.FetchPolicy`.
    pub policy: FetchPolicy,
}

/// SqliteStore — `cortex.instance.SqliteStore`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteStore {
    /// `path` — `Optional<String>`.
    pub path: Option<String>,
}

/// StoreSpec — `cortex.instance.StoreSpec`: one of a fixed set of shapes, tagged on the wire by `backend`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreSpec {
    /// Tagged `postgres` — `cortex.instance.PostgresStore`.
    Postgres(PostgresStore),
    /// Tagged `sqlite` — `Optional<cortex.instance.SqliteStore>`.
    Sqlite(Option<SqliteStore>),
}

/// StructuredChild — `cortex.instance.StructuredChild`.
///
/// Every value satisfies `paging.stop_older_than_since != true` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredChild {
    /// `operation` — `String`.
    pub operation: String,
    /// `input` — `Json`.
    pub input: crate::json::Value,
    /// `records` — `String`.
    pub records: String,
    /// `paging` — `Optional<cortex.instance.Paging>`.
    pub paging: Option<Paging>,
    /// `mapping` — `cortex.instance.RecordMapping`.
    pub mapping: RecordMapping,
    /// `time` — `Optional<String>`.
    pub time: Option<String>,
    /// `parent` — `String`.
    pub parent: String,
}

/// StructuredConnectors — `cortex.instance.StructuredConnectors`.
///
/// Every value satisfies `paging.stop_older_than_since != true` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `(defined(connection) or defined(profile))` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredConnectors {
    /// `adapter` — `String`.
    pub adapter: String,
    /// `connection` — `Optional<String>`.
    pub connection: Option<String>,
    /// `profile` — `Optional<String>`.
    pub profile: Option<String>,
    /// `operation` — `String`.
    pub operation: String,
    /// `inputs` — `List<Json>`.
    pub inputs: Vec<crate::json::Value>,
    /// `paging` — `Optional<cortex.instance.Paging>`.
    pub paging: Option<Paging>,
    /// `children` — `Optional<List<cortex.instance.StructuredChild>>`.
    pub children: Option<Vec<StructuredChild>>,
    /// `links` — `Optional<List<cortex.instance.CompareLink>>`.
    pub links: Option<Vec<CompareLink>>,
}

/// StructuredFiles — `cortex.instance.StructuredFiles`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredFiles {
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
    /// `glob` — `String`.
    pub glob: String,
}

/// StructuredInput — `cortex.instance.StructuredInput`: one of a fixed set of shapes, tagged on the wire by `from`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuredInput {
    /// Tagged `connectors` — `cortex.instance.StructuredConnectors`.
    Connectors(StructuredConnectors),
    /// Tagged `files` — `cortex.instance.StructuredFiles`.
    Files(StructuredFiles),
}

/// StructuredSource — `cortex.instance.StructuredSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredSource {
    /// `input` — `cortex.instance.StructuredInput`.
    pub input: StructuredInput,
    /// `records` — `String`.
    pub records: String,
    /// `mapping` — `cortex.instance.RecordMapping`.
    pub mapping: RecordMapping,
    /// `dropped` — `Optional<cortex.instance.DropPolicy>`.
    pub dropped: Option<DropPolicy>,
}

/// TimeRange — `cortex.instance.TimeRange`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRange {
    /// `day`.
    Day,
    /// `week`.
    Week,
    /// `month`.
    Month,
    /// `year`.
    Year,
}

/// TypeMerge — `cortex.instance.TypeMerge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeMerge {
    /// `types` — `List<String>`.
    pub types: Vec<String>,
    /// `into` — `String`.
    pub into: String,
}

/// TypeRemoval — `cortex.instance.TypeRemoval`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRemoval {
    /// `name` — `String`.
    pub name: String,
}

/// TypeSplit — `cortex.instance.TypeSplit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSplit {
    /// `name` — `String`.
    pub name: String,
    /// `into` — `List<String>`.
    pub into: Vec<String>,
}

/// WebInput — `cortex.instance.WebInput`: one of a fixed set of shapes, tagged on the wire by `input`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebInput {
    /// Tagged `search` — `cortex.instance.SearchInput`.
    Search(SearchInput),
    /// Tagged `sites` — `cortex.instance.SitesInput`.
    Sites(SitesInput),
}

/// WebMode — `cortex.instance.WebMode`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebMode {
    /// `Pages`.
    Pages,
    /// `Crawl`.
    Crawl,
}

/// WebSource — `cortex.instance.WebSource`.
///
/// Every value satisfies `(defined(connection) or defined(profile))` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSource {
    /// `adapter` — `Optional<String>`.
    pub adapter: Option<String>,
    /// `connection` — `Optional<String>`.
    pub connection: Option<String>,
    /// `profile` — `Optional<String>`.
    pub profile: Option<String>,
    /// `input` — `cortex.instance.WebInput`.
    pub input: WebInput,
}

/// What EvalRun — `cortex.instance.EvalRun` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`EvalRun<S>`], and at a boundary by [`EvalRunSnapshot::state`].
///
/// Every value satisfies `cases >= 0` — checked by [`EvalRunData::broken_invariant`].
/// Every value satisfies `passed >= 0` — checked by [`EvalRunData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalRunData {
    /// The identity: `stamp` — `String`.
    pub stamp: String,
    /// `instance_name` — `cortex.instance.InstanceName`.
    ///
    /// Carries `eval_runs`: `cortex.instance.Instance` owns many `cortex.instance.EvalRun`.
    pub instance_name: InstanceName,
    /// `runner` — `cortex.instance.EvalRunner`.
    pub runner: EvalRunner,
    /// `cases` — `Integer`.
    pub cases: i64,
    /// `passed` — `Integer`.
    pub passed: i64,
    /// `cost_usd` — `Optional<Decimal>`.
    pub cost_usd: Option<crate::primitives::Decimal>,
}

impl EvalRunData {
    /// The first declared invariant of `cortex.instance.EvalRun` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.cases)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("cases >= 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.passed)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("passed >= 0");
        }
        None
    }
}

/// The states of `cortex.instance.EvalRun`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](eval_run_state::Marker), so [`EvalRun<S>`](EvalRun) can only ever rest in a real state.
pub mod eval_run_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `EvalRun`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::EvalRunState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::EvalRunState = super::EvalRunState::Recorded;
    }
}

/// EvalRun — `cortex.instance.EvalRun` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`EvalRunSnapshot`]
/// and [`EvalRunSnapshot::refine`].
pub struct EvalRun<S: eval_run_state::Marker> {
    data: EvalRunData,
    state: core::marker::PhantomData<S>,
}

impl<S: eval_run_state::Marker> EvalRun<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> EvalRunState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &EvalRunData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> EvalRunData {
        self.data
    }
}

impl EvalRun<eval_run_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: EvalRunData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `cortex.instance.EvalRun` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`EvalRunSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalRunSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: EvalRunState,
    /// What it holds.
    pub data: EvalRunData,
}

/// An `EvalRun` in whichever declared state it was found.
pub enum AnyEvalRun {
    /// Resting in `Recorded`.
    Recorded(EvalRun<eval_run_state::Recorded>),
}

impl EvalRunSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `EvalRunState` cannot spell one.
    pub fn refine(self) -> AnyEvalRun {
        match self.state {
            EvalRunState::Recorded => AnyEvalRun::Recorded(EvalRun {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyEvalRun {
    /// The state, as the runtime value.
    pub fn state(&self) -> EvalRunState {
        match self {
            Self::Recorded(_) => EvalRunState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> EvalRunSnapshot {
        match self {
            Self::Recorded(instance) => EvalRunSnapshot {
                state: EvalRunState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Instance — `cortex.instance.Instance` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Instance<S>`], and at a boundary by [`InstanceSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceData {
    /// The identity: `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `description` — `String`.
    pub description: String,
    /// `model` — `String`.
    pub model: String,
    /// `ekr_version` — `String`.
    pub ekr_version: String,
    /// `seed_digest` — `String`.
    pub seed_digest: String,
}

/// The states of `cortex.instance.Instance`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](instance_state::Marker), so [`Instance<S>`](Instance) can only ever rest in a real state.
pub mod instance_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Active {}
        impl Sealed for super::Removed {}
    }

    /// A declared state of `Instance`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::InstanceState;
    }

    /// `Active`. Where a new instance starts.
    pub struct Active;

    impl Marker for Active {
        const STATE: super::InstanceState = super::InstanceState::Active;
    }

    /// `Removed`. Terminal: an instance may rest here forever.
    pub struct Removed;

    impl Marker for Removed {
        const STATE: super::InstanceState = super::InstanceState::Removed;
    }
}

/// Instance — `cortex.instance.Instance` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Active`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`InstanceSnapshot`]
/// and [`InstanceSnapshot::refine`].
pub struct Instance<S: instance_state::Marker> {
    data: InstanceData,
    state: core::marker::PhantomData<S>,
}

impl<S: instance_state::Marker> Instance<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> InstanceState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &InstanceData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> InstanceData {
        self.data
    }
}

impl Instance<instance_state::Active> {
    /// A new instance, resting in `Active` — the only state the lifecycle starts one in.
    pub fn new(data: InstanceData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Instance<instance_state::Active> {
    /// `remove` — `Active` → `Removed`. Taken by the `removed` outcome of `cortex.instance.RemoveInstance`.
    pub fn remove(self) -> Instance<instance_state::Removed> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `cortex.instance.Instance` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`InstanceSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: InstanceState,
    /// What it holds.
    pub data: InstanceData,
}

/// An `Instance` in whichever declared state it was found.
pub enum AnyInstance {
    /// Resting in `Active`.
    Active(Instance<instance_state::Active>),
    /// Resting in `Removed`.
    Removed(Instance<instance_state::Removed>),
}

impl InstanceSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `InstanceState` cannot spell one.
    pub fn refine(self) -> AnyInstance {
        match self.state {
            InstanceState::Active => AnyInstance::Active(Instance {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            InstanceState::Removed => AnyInstance::Removed(Instance {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyInstance {
    /// The state, as the runtime value.
    pub fn state(&self) -> InstanceState {
        match self {
            Self::Active(_) => InstanceState::Active,
            Self::Removed(_) => InstanceState::Removed,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> InstanceSnapshot {
        match self {
            Self::Active(instance) => InstanceSnapshot {
                state: InstanceState::Active,
                data: instance.into_data(),
            },
            Self::Removed(instance) => InstanceSnapshot {
                state: InstanceState::Removed,
                data: instance.into_data(),
            },
        }
    }
}

/// What SeenDocument — `cortex.instance.SeenDocument` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`SeenDocument<S>`], and at a boundary by [`SeenDocumentSnapshot::state`].
///
/// Every value satisfies `applied_at >= 0` — checked by [`SeenDocumentData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenDocumentData {
    /// The identity: `document_id` — `cortex.instance.DocumentId`.
    pub document_id: DocumentId,
    /// `source_id` — `cortex.instance.SourceId`.
    ///
    /// Carries `seen`: `cortex.instance.Source` owns many `cortex.instance.SeenDocument`.
    pub source_id: SourceId,
    /// `key` — `String`.
    pub key: String,
    /// `content_hash` — `Optional<String>`.
    pub content_hash: Option<String>,
    /// `unlinked_hash` — `Optional<String>`.
    pub unlinked_hash: Option<String>,
    /// `applied_at` — `Integer`.
    pub applied_at: i64,
}

impl SeenDocumentData {
    /// The first declared invariant of `cortex.instance.SeenDocument` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.applied_at)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("applied_at >= 0");
        }
        None
    }
}

/// The states of `cortex.instance.SeenDocument`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](seen_document_state::Marker), so [`SeenDocument<S>`](SeenDocument) can only ever rest in a real state.
pub mod seen_document_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Seen {}
    }

    /// A declared state of `SeenDocument`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SeenDocumentState;
    }

    /// `Seen`. Where a new instance starts.
    pub struct Seen;

    impl Marker for Seen {
        const STATE: super::SeenDocumentState = super::SeenDocumentState::Seen;
    }
}

/// SeenDocument — `cortex.instance.SeenDocument` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Seen`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SeenDocumentSnapshot`]
/// and [`SeenDocumentSnapshot::refine`].
pub struct SeenDocument<S: seen_document_state::Marker> {
    data: SeenDocumentData,
    state: core::marker::PhantomData<S>,
}

impl<S: seen_document_state::Marker> SeenDocument<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SeenDocumentState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SeenDocumentData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SeenDocumentData {
        self.data
    }
}

impl SeenDocument<seen_document_state::Seen> {
    /// A new instance, resting in `Seen` — the only state the lifecycle starts one in.
    pub fn new(data: SeenDocumentData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `cortex.instance.SeenDocument` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SeenDocumentSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeenDocumentSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SeenDocumentState,
    /// What it holds.
    pub data: SeenDocumentData,
}

/// An `SeenDocument` in whichever declared state it was found.
pub enum AnySeenDocument {
    /// Resting in `Seen`.
    Seen(SeenDocument<seen_document_state::Seen>),
}

impl SeenDocumentSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SeenDocumentState` cannot spell one.
    pub fn refine(self) -> AnySeenDocument {
        match self.state {
            SeenDocumentState::Seen => AnySeenDocument::Seen(SeenDocument {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySeenDocument {
    /// The state, as the runtime value.
    pub fn state(&self) -> SeenDocumentState {
        match self {
            Self::Seen(_) => SeenDocumentState::Seen,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SeenDocumentSnapshot {
        match self {
            Self::Seen(instance) => SeenDocumentSnapshot {
                state: SeenDocumentState::Seen,
                data: instance.into_data(),
            },
        }
    }
}

/// What Source — `cortex.instance.Source` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Source<S>`], and at a boundary by [`SourceSnapshot::state`].
///
/// Every value satisfies `runs >= 0` — checked by [`SourceData::broken_invariant`].
/// Every value satisfies `consecutive_failures >= 0` — checked by [`SourceData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceData {
    /// The identity: `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `instance_name` — `cortex.instance.InstanceName`.
    ///
    /// Carries `sources`: `cortex.instance.Instance` owns many `cortex.instance.Source`.
    pub instance_name: InstanceName,
    /// `name` — `String`.
    pub name: String,
    /// `kind` — `cortex.instance.SourceKind`.
    pub kind: SourceKind,
    /// `schedule` — `String`.
    pub schedule: String,
    /// `runs` — `Integer`.
    pub runs: i64,
    /// `consecutive_failures` — `Integer`.
    pub consecutive_failures: i64,
    /// `last_outcome` — `Optional<cortex.instance.RunOutcome>`.
    pub last_outcome: Option<RunOutcome>,
    /// `last_run_at` — `Optional<Timestamp>`.
    pub last_run_at: Option<crate::primitives::Timestamp>,
    /// `last_reason` — `Optional<String>`.
    pub last_reason: Option<String>,
}

impl SourceData {
    /// The first declared invariant of `cortex.instance.Source` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.runs)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("runs >= 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.consecutive_failures)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("consecutive_failures >= 0");
        }
        None
    }
}

/// The states of `cortex.instance.Source`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](source_state::Marker), so [`Source<S>`](Source) can only ever rest in a real state.
pub mod source_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Disabled {}
        impl Sealed for super::Enabled {}
    }

    /// A declared state of `Source`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SourceState;
    }

    /// `Disabled`.
    pub struct Disabled;

    impl Marker for Disabled {
        const STATE: super::SourceState = super::SourceState::Disabled;
    }

    /// `Enabled`. Where a new instance starts.
    pub struct Enabled;

    impl Marker for Enabled {
        const STATE: super::SourceState = super::SourceState::Enabled;
    }
}

/// Source — `cortex.instance.Source` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Enabled`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SourceSnapshot`]
/// and [`SourceSnapshot::refine`].
pub struct Source<S: source_state::Marker> {
    data: SourceData,
    state: core::marker::PhantomData<S>,
}

impl<S: source_state::Marker> Source<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SourceState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SourceData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SourceData {
        self.data
    }
}

impl Source<source_state::Enabled> {
    /// A new instance, resting in `Enabled` — the only state the lifecycle starts one in.
    pub fn new(data: SourceData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Source<source_state::Disabled> {
    /// `enable` — `Disabled` → `Enabled`. Taken by the `enabled` outcome of `cortex.instance.EnableSource`.
    pub fn enable(self) -> Source<source_state::Enabled> {
        Source {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Source<source_state::Enabled> {
    /// `disable` — `Enabled` → `Disabled`. Taken by the `disabled` outcome of `cortex.instance.RecordFailure`.
    pub fn disable(self) -> Source<source_state::Disabled> {
        Source {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `cortex.instance.Source` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SourceSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SourceState,
    /// What it holds.
    pub data: SourceData,
}

/// An `Source` in whichever declared state it was found.
pub enum AnySource {
    /// Resting in `Disabled`.
    Disabled(Source<source_state::Disabled>),
    /// Resting in `Enabled`.
    Enabled(Source<source_state::Enabled>),
}

impl SourceSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SourceState` cannot spell one.
    pub fn refine(self) -> AnySource {
        match self.state {
            SourceState::Disabled => AnySource::Disabled(Source {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SourceState::Enabled => AnySource::Enabled(Source {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySource {
    /// The state, as the runtime value.
    pub fn state(&self) -> SourceState {
        match self {
            Self::Disabled(_) => SourceState::Disabled,
            Self::Enabled(_) => SourceState::Enabled,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SourceSnapshot {
        match self {
            Self::Disabled(instance) => SourceSnapshot {
                state: SourceState::Disabled,
                data: instance.into_data(),
            },
            Self::Enabled(instance) => SourceSnapshot {
                state: SourceState::Enabled,
                data: instance.into_data(),
            },
        }
    }
}

/// Add an eval case from a review — the input of `cortex.instance.AddEvalCase`.
///
/// Everything it can result in is [`AddEvalCaseOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddEvalCase {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `set` — `String`.
    pub set: String,
    /// `from_review` — `cortex.ask.ReviewId`.
    pub from_review: crate::ask::ReviewId,
}

/// Everything `cortex.instance.AddEvalCase` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddEvalCaseOutcome {
    /// `set-invalid` — externally decided (The set file exists and cannot be read or is not a cortex.eval-case/1 set).
    ///
    /// The set file is unchanged.
    SetInvalid {
        /// Why it was refused: `cortex.instance.EvalSetInvalid`.
        error: EvalSetInvalid,
    },
    /// `no-such-review` — externally decided (The instance's answer records hold no review item from_review names, or not its answer).
    ///
    /// The set file is unchanged.
    NoSuchReview {
        /// Why it was refused: `cortex.instance.EvalReviewNotFound`.
        error: EvalReviewNotFound,
    },
    /// `review-open` — externally decided (The review item from_review names is still open).
    ///
    /// The item has no expectations until a reviewer closes it; the set file is unchanged.
    ReviewOpen {
        /// Why it was refused: `cortex.instance.EvalReviewOpen`.
        error: EvalReviewOpen,
    },
    /// `no-expectations` — externally decided (The review item from_review names is closed with no expected fact and no expected node).
    ///
    /// A case with nothing expected passes every answer; the set file is unchanged and not created.
    NoExpectations {
        /// Why it was refused: `cortex.instance.EvalNoExpectations`.
        error: EvalNoExpectations,
    },
    /// `case-exists` — externally decided (The set already holds a case with the review id as its id or its review_id).
    ///
    /// The set file is unchanged.
    CaseExists {
        /// Why it was refused: `cortex.instance.EvalCaseExists`.
        error: EvalCaseExists,
    },
    /// `added` — otherwise.
    ///
    /// The set file ends with the new case; `id` is the case's id and `cases` counts the set's cases with it.
    Added {
        /// The `cortex.instance.EvalCaseAdded` this outcome publishes.
        eval_case_added: EvalCaseAdded,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing was read or written.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// Add a source — the input of `cortex.instance.AddSource`.
///
/// Everything it can result in is [`AddSourceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddSource {
    /// `instance_name` — `cortex.instance.InstanceName`.
    pub instance_name: InstanceName,
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `name` — `String`.
    pub name: String,
    /// `kind` — `cortex.instance.SourceKind`.
    pub kind: SourceKind,
    /// `schedule` — `String`.
    pub schedule: String,
}

/// Everything `cortex.instance.AddSource` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddSourceOutcome {
    /// `added` — otherwise.
    ///
    /// The source's timer is installed and enabled.
    Added {
        /// The `cortex.instance.SourceAdded` this outcome publishes.
        source_added: SourceAdded,
    },
}

/// Adopt an existing store — the input of `cortex.instance.AdoptInstance`.
///
/// Everything it can result in is [`AdoptInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptInstance {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `description` — `String`.
    pub description: String,
    /// `model` — `String`.
    pub model: String,
    /// `ekr_version` — `String`.
    pub ekr_version: String,
    /// `seed_digest` — `String`.
    pub seed_digest: String,
    /// `spec` — `cortex.instance.InstanceSpec`.
    pub spec: InstanceSpec,
    /// `store` — `String`.
    pub store: String,
}

/// Everything `cortex.instance.AdoptInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptInstanceOutcome {
    /// `name-taken` — for an identity a record already carries.
    ///
    /// Nothing was adopted.
    NameTaken {
        /// Why it was refused: `cortex.instance.NameTaken`.
        error: NameTaken,
    },
    /// `backend-mismatch` — externally decided (The store is not on the backend the spec's store names, or not the postgres configuration it names).
    ///
    /// Nothing was adopted.
    BackendMismatch {
        /// Why it was refused: `cortex.instance.BackendMismatch`.
        error: BackendMismatch,
    },
    /// `store-unreadable` — externally decided (The store cannot be read, or EKR cannot open it under the instance's host document).
    ///
    /// Nothing was adopted.
    StoreUnreadable {
        /// Why it was refused: `cortex.instance.StoreUnreadable`.
        error: StoreUnreadable,
    },
    /// `store-held` — externally decided (An active instance of the home already grows the PostgreSQL lineage, the same configuration and tenant).
    ///
    /// Nothing was adopted.
    StoreHeld {
        /// Why it was refused: `cortex.instance.StoreHeld`.
        error: StoreHeld,
    },
    /// `seed-types-missing` — externally decided (A node or edge type of the spec's seed is not in the store's ontology).
    ///
    /// Nothing was adopted.
    SeedTypesMissing {
        /// Why it was refused: `cortex.instance.SeedTypesMissing`.
        error: SeedTypesMissing,
    },
    /// `adopted` — otherwise.
    ///
    /// The store answers the head it had, `revision`; nothing was seeded, extracted or written to it. A SQLite store was copied into the instance directory, which grows the copy; a PostgreSQL store is used where it is. The sources are added by AddSource, one per source in the spec.
    Adopted {
        /// The `cortex.instance.InstanceAdopted` this outcome publishes.
        instance_adopted: InstanceAdopted,
    },
}

/// Apply approved schema proposals — the input of `cortex.instance.ApplySchemaProposals`.
///
/// Everything it can result in is [`ApplySchemaProposalsOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplySchemaProposals {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `stamp` — `String`.
    pub stamp: String,
}

/// Everything `cortex.instance.ApplySchemaProposals` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplySchemaProposalsOutcome {
    /// `no-such-run` — externally decided (The instance has no readable `schema/<stamp>/proposals.jsonl`).
    ///
    /// Nothing was applied.
    NoSuchRun {
        /// Why it was refused: `cortex.instance.SchemaRunNotFound`.
        error: SchemaRunNotFound,
    },
    /// `not-approved` — externally decided (The run holds no `approved` proposal).
    ///
    /// Nothing was applied.
    NotApproved {
        /// Why it was refused: `cortex.instance.SchemaProposalsNotApproved`.
        error: SchemaProposalsNotApproved,
    },
    /// `gate-failed` — externally decided (The store after the changes fails a check of the spec's gate).
    ///
    /// Nothing was published; every approved proposal is still `approved`, and the store's head is where it was (`postgres`) or back at the snapshot (`sqlite`).
    GateFailed {
        /// Why it was refused: `cortex.instance.SchemaGateFailed`.
        error: SchemaGateFailed,
    },
    /// `apply-failed` — externally decided (EKR cannot read the ontology, begin the stage, take the snapshot, mint an id or answer a transaction).
    ///
    /// Nothing was published; every approved proposal is still `approved`.
    ApplyFailed {
        /// Why it was refused: `cortex.instance.SchemaApplyFailed`.
        error: SchemaApplyFailed,
    },
    /// `store-outcome-unknown` — externally decided (EKR cannot say whether the stage's begin or publish took effect).
    ///
    /// Every approved proposal is still `approved`. The stage is kept in `stage.json`, and the next run of a source settles it.
    StoreOutcomeUnknown {
        /// Why it was refused: `cortex.instance.StoreOutcomeUnknown`.
        error: StoreOutcomeUnknown,
    },
    /// `applied` — otherwise.
    ///
    /// Every approved proposal is decided: `applied` with the revision and schema version it committed, or `refused`, `recorded-only` or `invalid` with why. `revision` is the store's head after the apply. Proposals still `pending` stay so.
    Applied {
        /// The `cortex.instance.SchemaProposalsApplied` this outcome publishes.
        schema_proposals_applied: SchemaProposalsApplied,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing was applied.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// Approve schema proposals — the input of `cortex.instance.ApproveSchemaProposals`.
///
/// Everything it can result in is [`ApproveSchemaProposalsOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproveSchemaProposals {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `stamp` — `String`.
    pub stamp: String,
    /// `only` — `List<Integer>`.
    pub only: Vec<i64>,
    /// `by` — `Optional<String>`.
    pub by: Option<String>,
}

/// Everything `cortex.instance.ApproveSchemaProposals` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApproveSchemaProposalsOutcome {
    /// `no-such-run` — externally decided (The instance has no readable `schema/<stamp>/proposals.jsonl`).
    ///
    /// Nothing was decided.
    NoSuchRun {
        /// Why it was refused: `cortex.instance.SchemaRunNotFound`.
        error: SchemaRunNotFound,
    },
    /// `not-pending` — externally decided (A proposal `only` names is not in the run or is not `pending`, or the run has no pending proposal).
    ///
    /// Nothing was decided.
    NotPending {
        /// Why it was refused: `cortex.instance.SchemaProposalsNotPending`.
        error: SchemaProposalsNotPending,
    },
    /// `approved` — otherwise.
    ///
    /// Each named proposal is `approved`; `decided` counts them, `pending` what the run still holds undecided. `decision` is `approved`.
    Approved {
        /// The `cortex.instance.SchemaProposalsDecided` this outcome publishes.
        schema_proposals_decided: SchemaProposalsDecided,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing was decided.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// Create an instance — the input of `cortex.instance.CreateInstance`.
///
/// Everything it can result in is [`CreateInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateInstance {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `description` — `String`.
    pub description: String,
    /// `model` — `String`.
    pub model: String,
    /// `ekr_version` — `String`.
    pub ekr_version: String,
    /// `seed_digest` — `String`.
    pub seed_digest: String,
    /// `spec` — `cortex.instance.InstanceSpec`.
    pub spec: InstanceSpec,
}

/// Everything `cortex.instance.CreateInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateInstanceOutcome {
    /// `name-taken` — for an identity a record already carries.
    ///
    /// Nothing was created.
    NameTaken {
        /// Why it was refused: `cortex.instance.NameTaken`.
        error: NameTaken,
    },
    /// `connection-missing` — externally decided (Connectors lists no live connection for a connection id a source or the postgres store of the spec names, or not exactly one live connection for a profile one names).
    ///
    /// Nothing was created.
    ConnectionMissing {
        /// Why it was refused: `cortex.instance.ConnectionMissing`.
        error: ConnectionMissing,
    },
    /// `seed-refused` — externally decided (EKR refuses the minimal seed, the ekr-seed/2 file or the seed schema document).
    ///
    /// Nothing was created.
    SeedRefused {
        /// Why it was refused: `cortex.instance.SeedRefused`.
        error: SeedRefused,
    },
    /// `partial` — externally decided (The seed extraction stops before every seed document is extracted: the budget is spent, or a model call or an apply fails).
    ///
    /// As `created`, but some seed documents were not extracted. They are not recorded as seen, so `cortex run <name>/seed` extracts them.
    Partial {
        /// The `cortex.instance.InstanceCreated` this outcome publishes.
        instance_created: InstanceCreated,
    },
    /// `created` — otherwise.
    ///
    /// The store is seeded, the seed schema applied and the seed documents extracted into it. The viewer runs on its port. The sources are added by AddSource, one per source in the spec.
    Created {
        /// The `cortex.instance.InstanceCreated` this outcome publishes.
        instance_created: InstanceCreated,
    },
}

/// Enable a source — the input of `cortex.instance.EnableSource`.
///
/// Everything it can result in is [`EnableSourceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnableSource {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
}

/// Everything `cortex.instance.EnableSource` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnableSourceOutcome {
    /// `enabled` — otherwise.
    ///
    /// The failure count is reset and the timer is enabled again.
    Enabled {
        /// The `cortex.instance.SourceEnabled` this outcome publishes.
        source_enabled: SourceEnabled,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The source was not disabled.
    WrongState {
        /// Why it was refused: `cortex.instance.SourceNotDisabled`.
        error: SourceNotDisabled,
    },
    /// `no-such-source` — for an identity no record carries.
    ///
    /// Nothing changed.
    NoSuchSource {
        /// Why it was refused: `cortex.instance.SourceNotFound`.
        error: SourceNotFound,
    },
}

/// Measure fact quality — the input of `cortex.instance.MeasureQuality`.
///
/// Everything it can result in is [`MeasureQualityOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureQuality {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `sample` — `Integer`.
    pub sample: i64,
}

/// Everything `cortex.instance.MeasureQuality` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureQualityOutcome {
    /// `sample-failed` — externally decided (EKR cannot draw the sample, its size is outside 1 to 1000 or the store cannot be read).
    ///
    /// No model was asked and nothing was written.
    SampleFailed {
        /// Why it was refused: `cortex.instance.SampleFailed`.
        error: SampleFailed,
    },
    /// `judge-failed` — externally decided (A judge's model call fails, times out, answers no valid verdicts or finds the budget or the daily ceiling spent, or the redaction policy refuses what the judge would be shown).
    ///
    /// No pass rate was written; the verdicts of the batches judged before stay.
    JudgeFailed {
        /// Why it was refused: `cortex.instance.JudgeFailed`.
        error: JudgeFailed,
    },
    /// `measured` — otherwise.
    ///
    /// Every sampled fact was judged. `judged` counts them, `passed` those judged `yes` and `unclear` those the judge could not tell, which count as failed. `rate` is the pass rate, absent when the store holds no fact to draw; `lower` and `upper` are its Wilson interval at 95 %, 0 and 1 then. `cost_usd` is what the judge's model calls cost in USD, absent as soon as one answer carried no cost and never 0 for it. `revision` and `seed` draw the same sample again.
    Measured {
        /// The `cortex.instance.QualityMeasured` this outcome publishes.
        quality_measured: QualityMeasured,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing was measured.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// Propose schema changes — the input of `cortex.instance.ProposeSchemaChanges`.
///
/// Everything it can result in is [`ProposeSchemaChangesOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposeSchemaChanges {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `sample` — `Integer`.
    pub sample: i64,
    /// `dry_run` — `Boolean`.
    pub dry_run: bool,
    /// `review` — `Boolean`.
    pub review: bool,
}

/// Everything `cortex.instance.ProposeSchemaChanges` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposeSchemaChangesOutcome {
    /// `sample-failed` — externally decided (EKR cannot draw the sample, its size is outside 1 to 1000 or the store cannot be read).
    ///
    /// No model was asked and nothing was written.
    SampleFailed {
        /// Why it was refused: `cortex.instance.SampleFailed`.
        error: SampleFailed,
    },
    /// `propose-failed` — externally decided (A model call fails, times out, answers no valid proposals or finds the budget or the daily ceiling spent, the redaction policy refuses what the model would be shown, or EKR cannot read the ontology).
    ///
    /// Nothing was applied and no `proposals.jsonl` was written.
    ProposeFailed {
        /// Why it was refused: `cortex.instance.ProposeFailed`.
        error: ProposeFailed,
    },
    /// `proposed` — when `dry_run`.
    ///
    /// A dry run: every proposal not dropped is recorded and none is applied; `applied` and `refused` are 0, and a change EKR would be asked to apply is `dry-run`. `cost_usd` is what the model calls cost in USD, absent as soon as one answer carried no cost and never 0 for it.
    Proposed {
        /// The `cortex.instance.SchemaChangesProposed` this outcome publishes.
        schema_changes_proposed: SchemaChangesProposed,
    },
    /// `reviewed` — when `review`.
    ///
    /// A review run: every proposal not dropped is recorded `pending` and none is applied, except one whose name holds a placeholder of the redaction policy, `invalid`; `pending` counts them and `applied`, `refused` and `recorded_only` are 0. `stamp` names the run for `approve`, `reject` and `apply`. `cost_usd` is what the model calls cost in USD, absent as soon as one answer carried no cost and never 0 for it.
    Reviewed {
        /// The `cortex.instance.SchemaChangesProposed` this outcome publishes.
        schema_changes_proposed: SchemaChangesProposed,
    },
    /// `applied` — otherwise.
    ///
    /// Every proposal EKR accepted is committed, each as its own schema transaction and schema version; `applied` counts them, 0 when it accepted none. `revision` is the head the sample was drawn at, so `ekr ontology --at <revision>` still answers the schema before. `cost_usd` is what the model calls cost in USD, absent as soon as one answer carried no cost and never 0 for it.
    Applied {
        /// The `cortex.instance.SchemaChangesProposed` this outcome publishes.
        schema_changes_proposed: SchemaChangesProposed,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing was proposed.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// Record a failed run — the input of `cortex.instance.RecordFailure`.
///
/// Everything it can result in is [`RecordFailureOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFailure {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `cortex.instance.RecordFailure` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordFailureOutcome {
    /// `disabled` — when the existing subject's stored fields satisfy `consecutive_failures >= 1`.
    ///
    /// The second failure in a row disables the source's timer.
    Disabled {
        /// The `cortex.instance.SourceDisabled` this outcome publishes.
        source_disabled: SourceDisabled,
    },
    /// `counted` — otherwise.
    ///
    /// The failure is counted; the timer stays enabled.
    Counted {
        /// The `cortex.instance.RunFailed` this outcome publishes.
        run_failed: RunFailed,
    },
    /// `already-disabled` — from a state no declared move starts in.
    ///
    /// The source was already disabled; nothing was counted.
    AlreadyDisabled {
        /// Why it was refused: `cortex.instance.SourceDisabledError`.
        error: SourceDisabledError,
    },
    /// `no-such-source` — for an identity no record carries.
    ///
    /// Nothing was counted.
    NoSuchSource {
        /// Why it was refused: `cortex.instance.SourceNotFound`.
        error: SourceNotFound,
    },
}

/// Reject schema proposals — the input of `cortex.instance.RejectSchemaProposals`.
///
/// Everything it can result in is [`RejectSchemaProposalsOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectSchemaProposals {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `stamp` — `String`.
    pub stamp: String,
    /// `only` — `List<Integer>`.
    pub only: Vec<i64>,
    /// `reason` — `String`.
    pub reason: String,
    /// `by` — `Optional<String>`.
    pub by: Option<String>,
}

/// Everything `cortex.instance.RejectSchemaProposals` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectSchemaProposalsOutcome {
    /// `no-such-run` — externally decided (The instance has no readable `schema/<stamp>/proposals.jsonl`).
    ///
    /// Nothing was decided.
    NoSuchRun {
        /// Why it was refused: `cortex.instance.SchemaRunNotFound`.
        error: SchemaRunNotFound,
    },
    /// `not-pending` — externally decided (A proposal `only` names is not in the run or is not `pending`, the run has no pending proposal, or the reason is blank).
    ///
    /// Nothing was decided.
    NotPending {
        /// Why it was refused: `cortex.instance.SchemaProposalsNotPending`.
        error: SchemaProposalsNotPending,
    },
    /// `rejected` — otherwise.
    ///
    /// Each named proposal is `rejected` with the reason; `decided` counts them, `pending` what the run still holds undecided. `decision` is `rejected`.
    Rejected {
        /// The `cortex.instance.SchemaProposalsDecided` this outcome publishes.
        schema_proposals_decided: SchemaProposalsDecided,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing was decided.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// Remove an instance — the input of `cortex.instance.RemoveInstance`.
///
/// Everything it can result in is [`RemoveInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveInstance {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
}

/// Everything `cortex.instance.RemoveInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveInstanceOutcome {
    /// `removed` — otherwise.
    ///
    /// The timers and the viewer are gone; the directory and the store stay.
    Removed {
        /// The `cortex.instance.InstanceRemoved` this outcome publishes.
        instance_removed: InstanceRemoved,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The instance was already removed.
    WrongState {
        /// Why it was refused: `cortex.instance.InstanceNotActive`.
        error: InstanceNotActive,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing changed.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// Restore a snapshot — the input of `cortex.instance.RestoreSnapshot`.
///
/// Everything it can result in is [`RestoreSnapshotOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreSnapshot {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `snapshot` — `String`.
    pub snapshot: String,
}

/// Everything `cortex.instance.RestoreSnapshot` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreSnapshotOutcome {
    /// `backend-unsupported` — externally decided (The instance's store is on the postgres backend, whose snapshot is the operator's database backup).
    ///
    /// Nothing was restored.
    BackendUnsupported {
        /// Why it was refused: `cortex.instance.RestoreUnsupported`.
        error: RestoreUnsupported,
    },
    /// `busy` — externally decided (Another cortex command holds the home's lock, or another connection holds the store's write lock for 5 seconds).
    ///
    /// Nothing was restored.
    Busy {
        /// Why it was refused: `cortex.instance.InstanceBusy`.
        error: InstanceBusy,
    },
    /// `no-such-snapshot` — externally decided (The instance directory holds no snapshot by this name).
    ///
    /// Nothing was restored.
    NoSuchSnapshot {
        /// Why it was refused: `cortex.instance.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `restored` — otherwise.
    ///
    /// The store and the instance state were snapshotted as `<ms>-before-restore`, then put back to the snapshot's copies: every source's seen state and the known entity names included, so the next runs apply again what the undone runs applied. A viewer that was running was stopped first and started again after.
    Restored {
        /// The `cortex.instance.SnapshotRestored` this outcome publishes.
        snapshot_restored: SnapshotRestored,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing was restored.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// Run an eval set — the input of `cortex.instance.RunEval`.
///
/// Everything it can result in is [`RunEvalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEval {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `set` — `String`.
    pub set: String,
    /// `runner` — `cortex.instance.EvalRunner`.
    pub runner: EvalRunner,
    /// `from` — `Optional<String>`.
    pub from: Option<String>,
}

/// Everything `cortex.instance.RunEval` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEvalOutcome {
    /// `set-invalid` — externally decided (The set cannot be read or is not a cortex.eval-case/1 set, or from is missing for replay or given for another runner).
    ///
    /// No model was asked and nothing was written.
    SetInvalid {
        /// Why it was refused: `cortex.instance.EvalSetInvalid`.
        error: EvalSetInvalid,
    },
    /// `runner-failed` — externally decided (The runner cannot answer a case, its model call fails or finds the budget or the daily ceiling spent, no ask service answers, or the replayed run recorded no answer for a case).
    ///
    /// No summary was written; the results of the cases answered before stay.
    RunnerFailed {
        /// Why it was refused: `cortex.instance.EvalRunnerFailed`.
        error: EvalRunnerFailed,
    },
    /// `evaluated` — otherwise.
    ///
    /// Every case was answered and scored. `cases` counts them and `passed` those that passed; `facts_expected` and `facts_found` count the expected fact strings and those the answers contain, `nodes_expected` and `nodes_found` the expected node ids and those the answers cite. `model_calls` counts the model calls, 0 for `replay`; `input_tokens` and `output_tokens` are their tokens, absent as soon as one answer carried no count; `cost_usd` is what they cost in USD, absent as soon as one answer carried no cost.
    Evaluated {
        /// The `cortex.instance.EvalRan` this outcome publishes.
        eval_ran: EvalRan,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing was asked or written.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// Run a source once — the input of `cortex.instance.RunSource`.
///
/// Everything it can result in is [`RunSourceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSource {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `fetch_only` — `Optional<Boolean>`.
    pub fetch_only: Option<bool>,
}

/// Everything `cortex.instance.RunSource` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunSourceOutcome {
    /// `disabled` — when the existing subject is in Disabled and `(not (defined(fetch_only)) or fetch_only == false)`.
    ///
    /// The source is disabled: nothing ran and nothing was counted, and the source's status records the outcome.
    Disabled {
        /// The `cortex.instance.RunSkipped` this outcome publishes.
        run_skipped: RunSkipped,
    },
    /// `fetch-only-failed` — externally decided (Connectors or the filesystem fails to deliver the source's documents in a fetch-only run) when eligible ((defined(fetch_only) and fetch_only == true)).
    ///
    /// Nothing was written: not the source's status, its seen documents or its window, and no failure is counted. The command line prints it as `fetch-failed`.
    FetchOnlyFailed {
        /// Why it was refused: `cortex.instance.FetchFailed`.
        error: FetchFailed,
    },
    /// `fetched` — when `(defined(fetch_only) and fetch_only == true)`.
    ///
    /// A fetch-only run: the source was fetched over the window a run would ask about, its documents masked and scrubbed as a run's are and compared with its seen documents by content hash. `new` counts keys no seen document carries; `changed` those a run would extract now; `held` those whose change `refresh_after_days` holds back; `unchanged` the rest. No model was asked, and nothing was written: not the store, the seen documents, the source's status or `cortex.log`; no snapshot or stage was taken and no run counted.
    Fetched {
        /// The `cortex.instance.SourceFetched` this outcome publishes.
        source_fetched: SourceFetched,
    },
    /// `fetch-failed` — externally decided (Connectors or the filesystem fails to deliver the source's documents; an invocation Connectors refuses with timeout at admission is retried once first).
    ///
    /// Nothing was applied; the source's status records the outcome and its reason, and the scheduler then sends RecordFailure.
    FetchFailed {
        /// Why it was refused: `cortex.instance.FetchFailed`.
        error: FetchFailed,
    },
    /// `extraction-failed` — externally decided (The model call fails, times out, exceeds the run budget before any batch, or returns no valid document; an error answer that cost zero is asked once more first, after a fixed wait).
    ///
    /// Nothing was applied; the source's status records the outcome and its reason, and the scheduler then sends RecordFailure. When the model answered an error, `reason` holds the error text it gave, masked for credential shapes as fetched text is and cut at a character boundary to a bound, and the batch's `claude/error.txt` holds the same text. An error answer with a cost above zero, or with no cost reported, is not asked again.
    ExtractionFailed {
        /// Why it was refused: `cortex.instance.ExtractionFailed`.
        error: ExtractionFailed,
    },
    /// `apply-refused` — externally decided (EKR refuses the merged extraction document).
    ///
    /// Nothing was applied; the source's status records the outcome and its reason, and the scheduler then sends RecordFailure.
    ApplyRefused {
        /// Why it was refused: `cortex.instance.ApplyRefused`.
        error: ApplyRefused,
    },
    /// `store-outcome-unknown` — externally decided (EKR answers that the publication outcome of a store write of the run (a stage verb or `apply-extraction`) is unknown).
    ///
    /// The source's status records the outcome and its reason, and the scheduler sends no RecordFailure: the store, not the source, failed, so the failure is not counted and never disables the source. The next run settles the stage; a gated `sqlite` run that kept a snapshot is undone from it; otherwise the next run extracts the batch whose apply was unknown again.
    StoreOutcomeUnknown {
        /// Why it was refused: `cortex.instance.StoreOutcomeUnknown`.
        error: StoreOutcomeUnknown,
    },
    /// `connection-unavailable` — externally decided (Connectors answers a retryable operational failure to every attempt of `connections revalidate` (or of the `connections list` read before it, which for a source naming a profile is also the read that resolves the profile) before the run's first fetch).
    ///
    /// Nothing was fetched or applied; the source's status records the outcome and its reason, naming the last error, and the scheduler sends no RecordFailure: Connectors, not the source, failed, so the failure is not counted and never disables the source. The next scheduled run starts as usual.
    ConnectionUnavailable {
        /// Why it was refused: `cortex.instance.ConnectionUnavailable`.
        error: ConnectionUnavailable,
    },
    /// `batch-too-large` — externally decided (`claude` answers that a part of several documents of an extraction batch is too long for the model's window, or that its answer was cut off at the output limit, after the batch was split 3 deep).
    ///
    /// The source's status records the outcome and its reason, which names the documents of the part that did not fit and the batch size used, and the scheduler sends no RecordFailure: the batch, not the source, failed, so the failure is not counted and never disables the source. The batches before it stay applied; the rest are extracted by a later run. A part of one document is not this outcome: the run leaves it out and goes on.
    BatchTooLarge {
        /// Why it was refused: `cortex.instance.BatchTooLarge`.
        error: BatchTooLarge,
    },
    /// `ran` — otherwise.
    ///
    /// Every new or changed document within the policy's limits was extracted and applied, in batches, until the run's budget was spent. `documents_new` 0 means nothing changed and no model was called. `daily_ceiling_reached` is true when the run stopped before a batch because the instance's spend of the UTC day reached `model.daily_ceiling_usd` or its tokens reached `model.daily_ceiling_tokens`: the batches before it are applied, the rest are extracted by a run on a later day. A run stopped because an answer's tokens could not be counted does not set it. `cost_counted` is false behind a model gateway, where `cost_usd` is as reported and bounded nothing.
    Ran {
        /// The `cortex.instance.SourceRan` this outcome publishes.
        source_ran: SourceRan,
    },
    /// `no-such-source` — for an identity no record carries.
    ///
    /// Nothing ran.
    NoSuchSource {
        /// Why it was refused: `cortex.instance.SourceNotFound`.
        error: SourceNotFound,
    },
}

/// Update an instance — the input of `cortex.instance.UpdateInstance`.
///
/// Everything it can result in is [`UpdateInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInstance {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `description` — `String`.
    pub description: String,
    /// `model` — `String`.
    pub model: String,
    /// `spec` — `cortex.instance.InstanceSpec`.
    pub spec: InstanceSpec,
    /// `seed_digest` — `String`.
    pub seed_digest: String,
}

/// Everything `cortex.instance.UpdateInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateInstanceOutcome {
    /// `seed-change-refused` — when the existing subject's stored fields satisfy `seed_digest != input.seed_digest`.
    ///
    /// Nothing changed.
    SeedChangeRefused {
        /// Why it was refused: `cortex.instance.SeedChangeRefused`.
        error: SeedChangeRefused,
    },
    /// `not-active` — when the existing subject's stored fields satisfy `state == Removed`.
    ///
    /// The instance is removed; nothing changed.
    NotActive {
        /// Why it was refused: `cortex.instance.InstanceNotActive`.
        error: InstanceNotActive,
    },
    /// `updated` — otherwise.
    ///
    /// The frozen spec is replaced; sources, model and serve settings take effect on the next run. The viewer unit holds the connection id resolved at the last create, adopt or update: it is rewritten when the store's connection changed, or when its profile now resolves to another id.
    Updated {
        /// The `cortex.instance.InstanceUpdated` this outcome publishes.
        instance_updated: InstanceUpdated,
    },
    /// `no-such-instance` — for an identity no record carries.
    ///
    /// Nothing changed.
    NoSuchInstance {
        /// Why it was refused: `cortex.instance.InstanceNotFound`.
        error: InstanceNotFound,
    },
}

/// EvalCaseAdded — the event `cortex.instance.EvalCaseAdded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCaseAdded {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `set` — `String`.
    pub set: String,
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: crate::ask::ReviewId,
    /// `id` — `String`.
    pub id: String,
    /// `cases` — `Integer`.
    pub cases: i64,
}

/// EvalRan — the event `cortex.instance.EvalRan`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalRan {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `stamp` — `String`.
    pub stamp: String,
    /// `runner` — `cortex.instance.EvalRunner`.
    pub runner: EvalRunner,
    /// `cases` — `Integer`.
    pub cases: i64,
    /// `passed` — `Integer`.
    pub passed: i64,
    /// `facts_expected` — `Integer`.
    pub facts_expected: i64,
    /// `facts_found` — `Integer`.
    pub facts_found: i64,
    /// `nodes_expected` — `Integer`.
    pub nodes_expected: i64,
    /// `nodes_found` — `Integer`.
    pub nodes_found: i64,
    /// `model_calls` — `Integer`.
    pub model_calls: i64,
    /// `input_tokens` — `Optional<Integer>`.
    pub input_tokens: Option<i64>,
    /// `output_tokens` — `Optional<Integer>`.
    pub output_tokens: Option<i64>,
    /// `cost_usd` — `Optional<Decimal>`.
    pub cost_usd: Option<crate::primitives::Decimal>,
}

/// InstanceAdopted — the event `cortex.instance.InstanceAdopted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceAdopted {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `revision` — `Integer`.
    pub revision: i64,
    /// `view_port` — `Integer`.
    pub view_port: i64,
}

/// InstanceCreated — the event `cortex.instance.InstanceCreated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceCreated {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `view_port` — `Integer`.
    pub view_port: i64,
}

/// InstanceRemoved — the event `cortex.instance.InstanceRemoved`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRemoved {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
}

/// InstanceUpdated — the event `cortex.instance.InstanceUpdated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceUpdated {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
}

/// QualityMeasured — the event `cortex.instance.QualityMeasured`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualityMeasured {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `stamp` — `String`.
    pub stamp: String,
    /// `revision` — `Integer`.
    pub revision: i64,
    /// `seed` — `Integer`.
    pub seed: i64,
    /// `judged` — `Integer`.
    pub judged: i64,
    /// `passed` — `Integer`.
    pub passed: i64,
    /// `unclear` — `Integer`.
    pub unclear: i64,
    /// `rate` — `Optional<Decimal>`.
    pub rate: Option<crate::primitives::Decimal>,
    /// `lower` — `Decimal`.
    pub lower: crate::primitives::Decimal,
    /// `upper` — `Decimal`.
    pub upper: crate::primitives::Decimal,
    /// `cost_usd` — `Optional<Decimal>`.
    pub cost_usd: Option<crate::primitives::Decimal>,
}

/// RunFailed — the event `cortex.instance.RunFailed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFailed {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `reason` — `String`.
    pub reason: String,
}

/// RunSkipped — the event `cortex.instance.RunSkipped`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSkipped {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
}

/// SchemaChangesProposed — the event `cortex.instance.SchemaChangesProposed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaChangesProposed {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `stamp` — `String`.
    pub stamp: String,
    /// `revision` — `Integer`.
    pub revision: i64,
    /// `proposed` — `Integer`.
    pub proposed: i64,
    /// `applied` — `Integer`.
    pub applied: i64,
    /// `refused` — `Integer`.
    pub refused: i64,
    /// `recorded_only` — `Integer`.
    pub recorded_only: i64,
    /// `invalid` — `Integer`.
    pub invalid: i64,
    /// `pending` — `Integer`.
    pub pending: i64,
    /// `dropped` — `Integer`.
    pub dropped: i64,
    /// `cost_usd` — `Optional<Decimal>`.
    pub cost_usd: Option<crate::primitives::Decimal>,
}

/// SchemaProposalsApplied — the event `cortex.instance.SchemaProposalsApplied`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposalsApplied {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `stamp` — `String`.
    pub stamp: String,
    /// `applied` — `Integer`.
    pub applied: i64,
    /// `refused` — `Integer`.
    pub refused: i64,
    /// `recorded_only` — `Integer`.
    pub recorded_only: i64,
    /// `invalid` — `Integer`.
    pub invalid: i64,
    /// `revision` — `Integer`.
    pub revision: i64,
}

/// SchemaProposalsDecided — the event `cortex.instance.SchemaProposalsDecided`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposalsDecided {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `stamp` — `String`.
    pub stamp: String,
    /// `decision` — `String`.
    pub decision: String,
    /// `decided` — `Integer`.
    pub decided: i64,
    /// `pending` — `Integer`.
    pub pending: i64,
}

/// SnapshotRestored — the event `cortex.instance.SnapshotRestored`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRestored {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `snapshot` — `String`.
    pub snapshot: String,
}

/// SourceAdded — the event `cortex.instance.SourceAdded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAdded {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `instance_name` — `cortex.instance.InstanceName`.
    pub instance_name: InstanceName,
}

/// SourceDisabled — the event `cortex.instance.SourceDisabled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDisabled {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `reason` — `String`.
    pub reason: String,
}

/// SourceEnabled — the event `cortex.instance.SourceEnabled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEnabled {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
}

/// SourceFetched — the event `cortex.instance.SourceFetched`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFetched {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `since` — `Timestamp`.
    pub since: crate::primitives::Timestamp,
    /// `until` — `Timestamp`.
    pub until: crate::primitives::Timestamp,
    /// `fetched` — `Integer`.
    pub fetched: i64,
    /// `new` — `Integer`.
    pub new: i64,
    /// `changed` — `Integer`.
    pub changed: i64,
    /// `held` — `Integer`.
    pub held: i64,
    /// `unchanged` — `Integer`.
    pub unchanged: i64,
    /// `unread` — `List<String>`.
    pub unread: Vec<String>,
}

/// SourceRan — the event `cortex.instance.SourceRan`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRan {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `documents_new` — `Integer`.
    pub documents_new: i64,
    /// `documents_applied` — `Integer`.
    pub documents_applied: i64,
    /// `cost_usd` — `Optional<Decimal>`.
    pub cost_usd: Option<crate::primitives::Decimal>,
    /// `cost_counted` — `Boolean`.
    pub cost_counted: bool,
    /// `daily_ceiling_reached` — `Boolean`.
    pub daily_ceiling_reached: bool,
    /// `model_calls` — `Integer`.
    pub model_calls: i64,
    /// `input_tokens` — `Optional<Integer>`.
    pub input_tokens: Option<i64>,
    /// `cache_read_input_tokens` — `Optional<Integer>`.
    pub cache_read_input_tokens: Option<i64>,
    /// `cache_creation_input_tokens` — `Optional<Integer>`.
    pub cache_creation_input_tokens: Option<i64>,
    /// `output_tokens` — `Optional<Integer>`.
    pub output_tokens: Option<i64>,
    /// `fixed_input_tokens` — `Optional<Integer>`.
    pub fixed_input_tokens: Option<i64>,
    /// `documents_input_tokens` — `Optional<Integer>`.
    pub documents_input_tokens: Option<i64>,
    /// `batch_chars` — `Integer`.
    pub batch_chars: i64,
    /// `splits` — `Integer`.
    pub splits: i64,
}

/// The declared error `cortex.instance.ApplyRefused`.
///
/// EKR refused the merged extraction document; nothing was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRefused {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.BackendMismatch`.
///
/// The store named for adoption is not a store on the backend the spec's `store` names: no SQLite database for `sqlite`, or not the `ekr.postgres/1` file `store.value.config` names for `postgres`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendMismatch {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.BatchTooLarge`.
///
/// A part of several documents of an extraction batch was still too long for the model's window, or its answer still cut off at the output limit, after the batch was split 3 deep. Nothing about the source was wrong; a later run extracts that part, and a smaller `model.batch_chars` makes its batches smaller. A single document that alone does not fit is not this error: the run leaves it out and goes on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchTooLarge {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.ConnectionMissing`.
///
/// Connectors lists no live connection with the id a source or the postgres store names, or not exactly one live connection with the profile it names. Run `connectors connections connect` for it, or revoke the extra ones, then try again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionMissing {
    /// `connection` — `String`.
    pub connection: String,
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.ConnectionUnavailable`.
///
/// Connectors answered a retryable operational failure (`kind` `operational`, `next_action` `retry_explicitly`) to every attempt of the run's revalidation of its connection, or of the `connections list` read that resolves a profile-named connection, at most 4 attempts with pauses of 5 s, 10 s and 20 s, before the run fetched any document. Nothing about the source was wrong: the failure is not counted against it, and the next run starts as usual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionUnavailable {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.EvalCaseExists`.
///
/// The set already holds the case of this review item, or a case with its id; one review item yields one case in a set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCaseExists {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: crate::ask::ReviewId,
}

/// The declared error `cortex.instance.EvalNoExpectations`.
///
/// The review item is closed but its reviewer wrote no expected fact and no expected node (an item closed before the answer records held expectations has none): a case with nothing expected passes every answer, so no case was added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalNoExpectations {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: crate::ask::ReviewId,
}

/// The declared error `cortex.instance.EvalReviewNotFound`.
///
/// The instance's answer records hold no review item with this id, or not the answer record it belongs to (a purge took them); no case was added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalReviewNotFound {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: crate::ask::ReviewId,
}

/// The declared error `cortex.instance.EvalReviewOpen`.
///
/// The review item is still open: its expected facts and nodes are written when a reviewer closes it, so it yields no case yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalReviewOpen {
    /// `review_id` — `cortex.ask.ReviewId`.
    pub review_id: crate::ask::ReviewId,
}

/// The declared error `cortex.instance.EvalRunnerFailed`.
///
/// The runner could not answer a case: the model call failed, timed out, answered no valid answer or found the budget or the instance's daily ceiling spent; no `cortex ask serve` answered; or the run `replay` names holds no recorded answer for a case. `results.jsonl` keeps the results of the cases answered before; no `summary.json` was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalRunnerFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.EvalSetInvalid`.
///
/// The eval set cannot be read, or is not a `cortex.eval-case/1` set: a case of another `format`, a case without an id or a question, two cases with one id, or no case; or `--from` is missing for `replay`, or given for another runner. No model was asked and nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalSetInvalid {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.ExtractionFailed`.
///
/// The model call failed or returned no valid extraction document; nothing was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.FetchFailed`.
///
/// Connectors or the filesystem could not deliver the source's documents (an outage, a refused operation). Nothing was applied and the source's seen state did not move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.InstanceBusy`.
///
/// Another cortex command holds the home's lock, or another connection held the store's write lock for 5 seconds; nothing was restored. A reader that holds the store open does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceBusy {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.InstanceNotActive`.
///
/// The instance is removed, so nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceNotActive {
    /// `state` — `cortex.instance.Instance.State`.
    pub state: InstanceState,
}

/// The declared error `cortex.instance.InstanceNotFound`.
///
/// No instance has this name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceNotFound {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
}

/// The declared error `cortex.instance.JudgeFailed`.
///
/// A judge's model call failed, timed out, answered no valid verdicts or found the budget or the instance's daily ceiling spent, or the redaction policy refused what the judge would be shown. `verdicts.jsonl` keeps the verdicts of the batches judged before; no `fact-quality.json` was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.NameTaken`.
///
/// The home already holds an instance with this name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameTaken {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
}

/// The declared error `cortex.instance.ProposeFailed`.
///
/// A model call for schema proposals failed, timed out, answered no valid proposals or found the budget or the instance's daily ceiling spent, the redaction policy refused what the model would be shown, or EKR could not read the store's ontology. Nothing was applied and no `proposals.jsonl` was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposeFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.RestoreUnsupported`.
///
/// The instance's store is on the postgres backend: its snapshot is the operator's database backup, which cortex neither takes nor restores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreUnsupported {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
}

/// The declared error `cortex.instance.SampleFailed`.
///
/// EKR could not draw the sample (`ekr sample`): the size is outside 1 to 1000, or the store cannot be read under the instance's host. No model was asked and nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.SchemaApplyFailed`.
///
/// EKR could not read the ontology, begin the stage, take the snapshot, mint an id or answer a transaction. Nothing was published: the stage was abandoned, or on `sqlite` the snapshot taken before was restored; every approved proposal is still `approved`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaApplyFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.SchemaGateFailed`.
///
/// The applied schema changes failed the spec's `gate`. Nothing was published: the stage was abandoned, or on `sqlite` the snapshot taken before was restored; every approved proposal is still `approved`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaGateFailed {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.SchemaProposalsNotApproved`.
///
/// `apply` found no approved proposal in the run: none was approved, or every approved one is applied or decided. Nothing was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposalsNotApproved {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.SchemaProposalsNotPending`.
///
/// `approve` or `reject` named a proposal the run does not have or that is already decided, or the run has no pending proposal left. A decided proposal is not decided again; nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaProposalsNotPending {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.SchemaRunNotFound`.
///
/// The instance has no schema run by this stamp: no `schema/<stamp>/proposals.jsonl`, or one that cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaRunNotFound {
    /// `stamp` — `String`.
    pub stamp: String,
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.SeedChangeRefused`.
///
/// An update may change sources, model and serve settings only; the seed of an existing store cannot change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedChangeRefused {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
}

/// The declared error `cortex.instance.SeedRefused`.
///
/// EKR refused the seed or the seed schema; no store was kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedRefused {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.SeedTypesMissing`.
///
/// A node or edge type the spec's seed declares is not in the store's ontology. `types` lists them, separated by a comma and a space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedTypesMissing {
    /// `types` — `String`.
    pub types: String,
}

/// The declared error `cortex.instance.SnapshotNotFound`.
///
/// The instance directory holds no snapshot by this name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotNotFound {
    /// `snapshot` — `String`.
    pub snapshot: String,
}

/// The declared error `cortex.instance.SourceDisabledError`.
///
/// The source is disabled after repeated failures; enable it first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDisabledError {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
}

/// The declared error `cortex.instance.SourceNotDisabled`.
///
/// The source is already enabled, so nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceNotDisabled {
    /// `state` — `cortex.instance.Source.State`.
    pub state: SourceState,
}

/// The declared error `cortex.instance.SourceNotFound`.
///
/// No source has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceNotFound {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
}

/// The declared error `cortex.instance.StoreHeld`.
///
/// An active instance of the home, `name`, already grows this PostgreSQL lineage: the same `ekr.postgres/1` file and tenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreHeld {
    /// `name` — `String`.
    pub name: String,
}

/// The declared error `cortex.instance.StoreOutcomeUnknown`.
///
/// EKR could not say whether a store write of the run took effect (`ekr stage begin`, `ekr stage publish`, `ekr apply-extraction` or the settling of an earlier run's stage answered that the publication outcome is unknown). Nothing about the source was wrong; the next run settles the stage, or extracts the batch again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOutcomeUnknown {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.StoreUnreadable`.
///
/// The store named for adoption cannot be read, or EKR cannot open it under the instance's host document (another tenant or authority than the store was seeded under, or no seed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreUnreadable {
    /// `reason` — `String`.
    pub reason: String,
}

/// Instances — one row of the view `cortex.instance.Instances`.
///
/// Projects `cortex.instance.Instance` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instances {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
    /// `description` — `String`.
    pub description: String,
    /// `model` — `String`.
    pub model: String,
    /// `ekr_version` — `String`.
    pub ekr_version: String,
    /// `seed_digest` — `String`.
    pub seed_digest: String,
    /// `state` — `cortex.instance.Instance.State`.
    pub state: InstanceState,
}

/// Sources — one row of the view `cortex.instance.Sources`.
///
/// Projects `cortex.instance.Source` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sources {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `instance_name` — `cortex.instance.InstanceName`.
    pub instance_name: InstanceName,
    /// `name` — `String`.
    pub name: String,
    /// `kind` — `cortex.instance.SourceKind`.
    pub kind: SourceKind,
    /// `schedule` — `String`.
    pub schedule: String,
    /// `runs` — `Integer`.
    pub runs: i64,
    /// `consecutive_failures` — `Integer`.
    pub consecutive_failures: i64,
    /// `state` — `cortex.instance.Source.State`.
    pub state: SourceState,
    /// `last_outcome` — `Optional<cortex.instance.RunOutcome>`.
    pub last_outcome: Option<RunOutcome>,
    /// `last_run_at` — `Optional<Timestamp>`.
    pub last_run_at: Option<crate::primitives::Timestamp>,
    /// `last_reason` — `Optional<String>`.
    pub last_reason: Option<String>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `cortex.instance.AddEvalCase` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AddEvalCaseBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.AddEvalCase`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn add_eval_case(&mut self, input: super::AddEvalCase) -> Result<super::AddEvalCaseOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.AddSource` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AddSourceBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.AddSource`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn add_source(&mut self, input: super::AddSource) -> Result<super::AddSourceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.AdoptInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AdoptInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.AdoptInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn adopt_instance(&mut self, input: super::AdoptInstance) -> Result<super::AdoptInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.ApplySchemaProposals` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ApplySchemaProposalsBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.ApplySchemaProposals`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn apply_schema_proposals(&mut self, input: super::ApplySchemaProposals) -> Result<super::ApplySchemaProposalsOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.ApproveSchemaProposals` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ApproveSchemaProposalsBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.ApproveSchemaProposals`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn approve_schema_proposals(&mut self, input: super::ApproveSchemaProposals) -> Result<super::ApproveSchemaProposalsOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.CreateInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CreateInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.CreateInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn create_instance(&mut self, input: super::CreateInstance) -> Result<super::CreateInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.EnableSource` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait EnableSourceBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.EnableSource`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn enable_source(&mut self, input: super::EnableSource) -> Result<super::EnableSourceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.MeasureQuality` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait MeasureQualityBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.MeasureQuality`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn measure_quality(&mut self, input: super::MeasureQuality) -> Result<super::MeasureQualityOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.ProposeSchemaChanges` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ProposeSchemaChangesBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.ProposeSchemaChanges`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn propose_schema_changes(&mut self, input: super::ProposeSchemaChanges) -> Result<super::ProposeSchemaChangesOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.RecordFailure` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a subject predicate choosing between a move and an update.
    ///
    /// Contract: given `cortex.instance.RecordFailure` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `disabled` when the existing subject's stored fields satisfy `consecutive_failures >= 1`, takes `disable` of `cortex.instance.Source`, emits `cortex.instance.SourceDisabled`; `counted` otherwise, updates `cortex.instance.Source`, emits `cortex.instance.RunFailed`; `already-disabled` from a state no declared move starts in, error `cortex.instance.SourceDisabledError`; `no-such-source` for an identity no record carries, error `cortex.instance.SourceNotFound`.
    pub trait RecordFailureBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.RecordFailure`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn record_failure(&mut self, input: super::RecordFailure) -> Result<super::RecordFailureOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.RejectSchemaProposals` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RejectSchemaProposalsBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.RejectSchemaProposals`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn reject_schema_proposals(&mut self, input: super::RejectSchemaProposals) -> Result<super::RejectSchemaProposalsOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.RemoveInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RemoveInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.RemoveInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn remove_instance(&mut self, input: super::RemoveInstance) -> Result<super::RemoveInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.RestoreSnapshot` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RestoreSnapshotBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.RestoreSnapshot`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn restore_snapshot(&mut self, input: super::RestoreSnapshot) -> Result<super::RestoreSnapshotOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.RunEval` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RunEvalBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.RunEval`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn run_eval(&mut self, input: super::RunEval) -> Result<super::RunEvalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.RunSource` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a refusal that compensates (`compensates: true`), on `fetch-failed`.
    ///
    /// Contract: given `cortex.instance.RunSource` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `disabled` when the existing subject is in Disabled and `(not (defined(fetch_only)) or fetch_only == false)`, updates `cortex.instance.Source`, emits `cortex.instance.RunSkipped`; `fetch-only-failed` externally decided (Connectors or the filesystem fails to deliver the source's documents in a fetch-only run) when eligible ((defined(fetch_only) and fetch_only == true)), error `cortex.instance.FetchFailed`; `fetched` when `(defined(fetch_only) and fetch_only == true)`, updates `cortex.instance.Source`, emits `cortex.instance.SourceFetched`; `fetch-failed` externally decided (Connectors or the filesystem fails to deliver the source's documents; an invocation Connectors refuses with timeout at admission is retried once first), updates `cortex.instance.Source`, error `cortex.instance.FetchFailed`; `extraction-failed` externally decided (The model call fails, times out, exceeds the run budget before any batch, or returns no valid document; an error answer that cost zero is asked once more first, after a fixed wait), updates `cortex.instance.Source`, error `cortex.instance.ExtractionFailed`; `apply-refused` externally decided (EKR refuses the merged extraction document), updates `cortex.instance.Source`, error `cortex.instance.ApplyRefused`; `store-outcome-unknown` externally decided (EKR answers that the publication outcome of a store write of the run (a stage verb or `apply-extraction`) is unknown), updates `cortex.instance.Source`, error `cortex.instance.StoreOutcomeUnknown`; `connection-unavailable` externally decided (Connectors answers a retryable operational failure to every attempt of `connections revalidate` (or of the `connections list` read before it, which for a source naming a profile is also the read that resolves the profile) before the run's first fetch), updates `cortex.instance.Source`, error `cortex.instance.ConnectionUnavailable`; `batch-too-large` externally decided (`claude` answers that a part of several documents of an extraction batch is too long for the model's window, or that its answer was cut off at the output limit, after the batch was split 3 deep), updates `cortex.instance.Source`, error `cortex.instance.BatchTooLarge`; `ran` otherwise, updates `cortex.instance.Source`, emits `cortex.instance.SourceRan`; `no-such-source` for an identity no record carries, error `cortex.instance.SourceNotFound`.
    pub trait RunSourceBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.RunSource`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn run_source(&mut self, input: super::RunSource) -> Result<super::RunSourceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `cortex.instance.UpdateInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait UpdateInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `cortex.instance.UpdateInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn update_instance(&mut self, input: super::UpdateInstance) -> Result<super::UpdateInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `cortex.instance.Instances` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait InstancesQuery {
        /// Serves `cortex.instance.Instances` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn instances(&self) -> Result<Vec<super::Instances>, crate::obligation::UnmetObligation>;
    }

    /// The query `cortex.instance.Sources` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SourcesQuery {
        /// Serves `cortex.instance.Sources` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn sources(&self) -> Result<Vec<super::Sources>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl RecordFailureBehavior for Unimplemented {
        fn record_failure(&mut self, _input: super::RecordFailure) -> Result<super::RecordFailureOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "cortex.instance.RecordFailure" })
        }
    }

    impl RunSourceBehavior for Unimplemented {
        fn run_source(&mut self, _input: super::RunSource) -> Result<super::RunSourceOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "cortex.instance.RunSource" })
        }
    }
}
