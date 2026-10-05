// generated from cortex v1
// model digest b6310870f027779babbc552b01928ccc94a4334219f5aec56f4b20fd26352d85
// contract digest b949168dd73a8a82c270bfa858354cf5311edabd072da4e2c85eaf684999ade6
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

/// ConnectorsSource — `cortex.instance.ConnectorsSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorsSource {
    /// `adapter` — `String`.
    pub adapter: String,
    /// `connection` — `String`.
    pub connection: String,
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

/// DropPolicy — `cortex.instance.DropPolicy`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPolicy {
    /// `Keep`.
    Keep,
    /// `Supersede`.
    Supersede,
}

/// EkrPin — `cortex.instance.EkrPin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EkrPin {
    /// `version` — `String`.
    pub version: String,
    /// `bin` — `Optional<String>`.
    pub bin: Option<String>,
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
    /// `thread` — `Optional<String>`.
    pub thread: Option<String>,
    /// `filters` — `List<cortex.instance.RecordFilter>`.
    pub filters: Vec<RecordFilter>,
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
}

/// ModelBackend — `cortex.instance.ModelBackend`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelBackend {
    /// `Claude`.
    Claude,
    /// `Codex`.
    Codex,
}

/// ModelSpec — `cortex.instance.ModelSpec`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpec {
    /// `model` — `String`.
    pub model: String,
    /// `backend` — `Optional<cortex.instance.ModelBackend>`.
    pub backend: Option<ModelBackend>,
    /// `budget_usd` — `Decimal`.
    pub budget_usd: crate::primitives::Decimal,
    /// `timeout_s` — `Integer`.
    pub timeout_s: i64,
    /// `instructions` — `Optional<String>`.
    pub instructions: Option<String>,
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
}

/// PostgresStore — `cortex.instance.PostgresStore`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresStore {
    /// `config` — `String`.
    pub config: String,
}

/// PropertyMapping — `cortex.instance.PropertyMapping`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyMapping {
    /// `property` — `String`.
    pub property: String,
    /// `path` — `String`.
    pub path: String,
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

/// StructuredConnectors — `cortex.instance.StructuredConnectors`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredConnectors {
    /// `adapter` — `String`.
    pub adapter: String,
    /// `connection` — `String`.
    pub connection: String,
    /// `operation` — `String`.
    pub operation: String,
    /// `inputs` — `List<Json>`.
    pub inputs: Vec<crate::json::Value>,
    /// `paging` — `Optional<cortex.instance.Paging>`.
    pub paging: Option<Paging>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSource {
    /// `adapter` — `Optional<String>`.
    pub adapter: Option<String>,
    /// `connection` — `String`.
    pub connection: String,
    /// `input` — `cortex.instance.WebInput`.
    pub input: WebInput,
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
    /// `name-taken` — externally decided (The home already holds an instance with this name).
    ///
    /// Nothing was created.
    NameTaken {
        /// Why it was refused: `cortex.instance.NameTaken`.
        error: NameTaken,
    },
    /// `connection-missing` — externally decided (Connectors lists no live connection for a connection id a source of the spec names).
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

/// Run a source once — the input of `cortex.instance.RunSource`.
///
/// Everything it can result in is [`RunSourceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSource {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
}

/// Everything `cortex.instance.RunSource` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunSourceOutcome {
    /// `fetch-failed` — externally decided (Connectors or the filesystem fails to deliver the source's documents).
    ///
    /// Nothing was applied; the scheduler then sends RecordFailure.
    FetchFailed {
        /// Why it was refused: `cortex.instance.FetchFailed`.
        error: FetchFailed,
    },
    /// `extraction-failed` — externally decided (The model call fails, times out, exceeds the run budget before any batch, or returns no valid document).
    ///
    /// Nothing was applied; the scheduler then sends RecordFailure.
    ExtractionFailed {
        /// Why it was refused: `cortex.instance.ExtractionFailed`.
        error: ExtractionFailed,
    },
    /// `apply-refused` — externally decided (EKR refuses the merged extraction document).
    ///
    /// Nothing was applied; the scheduler then sends RecordFailure.
    ApplyRefused {
        /// Why it was refused: `cortex.instance.ApplyRefused`.
        error: ApplyRefused,
    },
    /// `ran` — when the existing subject is in Enabled.
    ///
    /// Every new or changed document within the policy's limits was extracted and applied, in batches, until the run's budget was spent. `documents_new` 0 means nothing changed and no model was called.
    Ran {
        /// The `cortex.instance.SourceRan` this outcome publishes.
        source_ran: SourceRan,
    },
    /// `disabled` — otherwise.
    ///
    /// The source is disabled; nothing ran.
    Disabled {
        /// Why it was refused: `cortex.instance.SourceDisabledError`.
        error: SourceDisabledError,
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
    /// `seed_changed` — `Boolean`.
    pub seed_changed: bool,
}

/// Everything `cortex.instance.UpdateInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateInstanceOutcome {
    /// `seed-change-refused` — when `seed_changed == true`.
    ///
    /// Nothing changed.
    SeedChangeRefused {
        /// Why it was refused: `cortex.instance.SeedChangeRefused`.
        error: SeedChangeRefused,
    },
    /// `updated` — when the existing subject is in Active.
    ///
    /// The frozen spec is replaced; sources, model and serve settings take effect on the next run.
    Updated {
        /// The `cortex.instance.InstanceUpdated` this outcome publishes.
        instance_updated: InstanceUpdated,
    },
    /// `not-active` — otherwise.
    ///
    /// The instance is removed; nothing changed.
    NotActive {
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

/// RunFailed — the event `cortex.instance.RunFailed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFailed {
    /// `source_id` — `cortex.instance.SourceId`.
    pub source_id: SourceId,
    /// `reason` — `String`.
    pub reason: String,
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
}

/// The declared error `cortex.instance.ApplyRefused`.
///
/// EKR refused the merged extraction document; nothing was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyRefused {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `cortex.instance.ConnectionMissing`.
///
/// Connectors lists no live connection with the id a source names. Run `connectors connections connect` for it, then try again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionMissing {
    /// `connection` — `String`.
    pub connection: String,
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

/// The declared error `cortex.instance.NameTaken`.
///
/// The home already holds an instance with this name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameTaken {
    /// `name` — `cortex.instance.InstanceName`.
    pub name: InstanceName,
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
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
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

    /// The behaviour `cortex.instance.RunSource` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by `when_subject_state:` beside `external:` in one command.
    ///
    /// Contract: given `cortex.instance.RunSource` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `fetch-failed` externally decided (Connectors or the filesystem fails to deliver the source's documents), error `cortex.instance.FetchFailed`; `extraction-failed` externally decided (The model call fails, times out, exceeds the run budget before any batch, or returns no valid document), error `cortex.instance.ExtractionFailed`; `apply-refused` externally decided (EKR refuses the merged extraction document), error `cortex.instance.ApplyRefused`; `ran` when the existing subject is in Enabled, updates `cortex.instance.Source`, emits `cortex.instance.SourceRan`; `disabled` otherwise, error `cortex.instance.SourceDisabledError`; `no-such-source` for an identity no record carries, error `cortex.instance.SourceNotFound`.
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
