# Changelog

Every user-visible change to cortex. Versions are the `version` in `Cargo.toml`; a release is the
tag `v<version>`. The process is in [AGENTS.md](AGENTS.md#cutting-a-release).

## Unreleased

## 0.9.1 — 2026-11-24

### Fixed

- A structured mapping may name one relation with several target types (`LINKS_TO` to `Page`
  and to `File`): the batch declares one edge type of that name whose target types are all of
  them, and commits. Before, the top mapping declared one edge type per relation entry, and EKR
  refused the batch with `extraction-name-duplicate`; child mappings already joined them (story
  `structured-relation-many-targets`).

## 0.9.0 — 2026-11-24

### Added

- `cortex update`, `cortex remove` and `cortex source enable` take `--no-wait` too: while another
  cortex command holds the home's lock they answer `busy` at once with exit status 9 and change
  nothing (an update leaves the frozen spec as it was), so an orchestrator that changes an
  instance's spec while a run may be in flight gets an answer at once instead of waiting minutes.
  Without the flag they wait for the lock, as before (story `update-no-wait`).
- `cortex run <instance>/<source> --extract-only` fetches and extracts as a run does and keeps
  each batch's extraction unapplied in the run directory (`extracted`, naming the `run`), writing
  nothing to the store, the seen state, the window or the source's status, so a caller can check
  the extracted facts before they are written. `cortex apply <instance> <run>` applies a kept
  extraction once, with a run's reconcile and refusal rules, snapshot or stage and gate, and
  advances the seen state then. A second apply answers `already-applied`; an extraction whose
  documents a run applied since answers `extraction-stale` and writes nothing (story
  `extract-then-apply`).
- `redaction.keep_store_types`: node types whose names in the store `RareName` never takes. A
  run reads the names and aliases of the store's nodes of those types once, before it applies
  anything, and shows them to the model as written, so a customer named once in a batch keeps
  its name and its facts. A node of an unlisted type, a `Person` among them, is hidden as before
  when its name is rare. Without the field, or with it empty, a run is as before (story
  `rare-name-keeps-store-types`).

### Changed

- **cortex pins EKR 0.0.43** (was 0.0.34): `cortex setup` installs it by default, and the
  examples and the CI name it. Run `cortex setup` and point an instance's `ekr.version` at
  `0.0.43`; cortex no longer reads an EKR 0.0.34 store outcome (story `ekr-pin-0-0-43`).
- cortex reads a store outcome by EKR's exit status and refusal name, not by its English text: a
  write EKR cannot say took effect is exit 3 `publication-outcome-unknown` (a run still fails as
  `store-outcome-unknown`, exit 4), and a store with no seed is exit 2 `ekr.views.NotSeeded`.
- A fetched web page's evidence is `!Url <its URL>`, which EKR 0.0.43 admits, instead of a
  `!HumanStatement` whose identity is the URL. Pages applied before keep their statement; both
  count as evidence of the page when a store is adopted.
- Development only, no change to the binary: CI validates the planning store with aep 0.80.2 (was
  0.75.0), and the store reads its lifecycles from the aep 0.80.2 release commit (was 0.68.0's);
  the store stays `aep.project/5` (story `aep-pin-0-80`).
- Development only, no change to the binary: the PostgreSQL tests share one PostgreSQL server per
  test run, each in a database of its own, instead of starting and removing a container each, and
  the test step of `task check` fails over 120 s by default (`TEST_BUDGET=enforce`).
- A run that `max_documents_per_run` capped reports `stopped: document-cap` instead of
  `stopped: null`, and every `ran` report carries `documents_left`, the wanted documents the run
  neither applied nor marked seen (`0` when it left none; `SourceRan` carries both). Before, a
  capped run looked like one that read everything, and a caller counted what was left with a
  second `--fetch-only` call.
- A batch directory under `runs/` keeps each part EKR refused in `refusals.json`: the part, the
  outcome EKR named and its reason. Under a `redaction` policy every value the policy finds in it
  is replaced by a placeholder of the batch, as in `prompt.txt` and `answer.json`. Before, a
  refusal's reason was in no file of the run directory under a policy.

### Fixed

- **`cortex quality` no longer counts an unanswered fact as a failure.** A fact the judge's answer
  gave no verdict was written `unclear` and counted as failed, so a judge that left many facts
  unanswered made the rate far too low. Such a fact is now asked once more, after the last
  batch, in a later judge call; still without a verdict it is `unjudged`: written so in
  `verdicts.jsonl`, not sent to `ekr fact-quality`, and outside the rate. `judged`, the rate's
  denominator, counts only the facts given a verdict, and the `measured` line and
  `QualityMeasured` carry the new `unjudged`. Each judge call keeps its request (`prompt.txt`)
  and the answer before any placeholder is restored (`answer.json`) in
  `quality/<stamp>/batch-<n>/`, which stayed empty before (story `quality-missing-verdicts`).
- A placeholder the model writes back without its brackets (`Name-3` for `[Name-3]`) is restored
  in a value or an alias, as in a type name. 0.8.0 left it as written and counted it
  `unrestored`, so a redaction gate on `unrestored` undid the run. One the batch's mapping does
  not hold still stays as written and is counted.
- A batch whose model call ends on `error_max_structured_output_retries` names the subtype, the
  number of turns and the last validation message of claude's `errors` list in the run's reason
  and in `claude/error.txt`, masked and cut as other error text. 0.8.0 wrote an empty
  `error.txt`.
- **Redaction takes time linear in the matches.** Each match a redaction class found was checked
  against every match found before it, so a text of many matches, such as links in brackets
  back to back, took time quadratic in their number (4x the links, 14x the time). The check is a
  binary search now; what is replaced is unchanged.
- **A commit or a version after `@` at the end of a text is no email address.** `Email` reads an
  address a provider cut before its top-level domain at the very end of a text (`jane.doe@exam`).
  It also read a document key such as `group/repo@3f2a9c1` there, and the model was shown
  `[Email-1]` for it. What is left of a cut address's domain now holds no digit, so a commit id
  or a version (`tool@1.4.2`) after `@` is kept.
- A batch that holds several replacing values of one entity and property with nothing stored to
  replace orders them by the earliest document each fact cites, the time EKR stores the fact
  from. 0.8.0 ordered a fact citing an early and a late document by the late one, applied it
  after a value it is older than, and EKR refused it with `invalid-supersession`, naming every
  document it cited in `rejected`. A replacement applied again that EKR still refuses that way
  is kept as history, as one of the batch's first apply is (`facts_history`).
- A `web` source reports what the provider did not read. Each URL in a `websearch.fetch` or
  `websearch.crawl` answer's `failed` list, and an answer that says `complete: false`, is named in
  fetch-only's `unread` and in the run's `stopped`, with the provider's reason or truncation
  causes. A search short of `max_results` alone is not named. 0.8.0 ignored both and reported a
  run whose only URL failed as a plain `ran` (review of 0.7.2, finding 7).
- A web page's description is cut, with its title, to what the 16,384-byte evidence payload
  holds, leaving the page's text its own length or 8,192 bytes. The prompt, the evidence and the
  stored `WebPage` `description` hold the same text. 0.8.0 showed the model and stored a long
  description whose tail its evidence did not hold (review of 0.7.2, finding 10).
- A `structured` record whose mapping changed is applied again at the next run, at once and
  whatever `refresh_after_days`, its text changed or not. 0.8.0 answered `cortex update` with
  `updated` and left every unchanged record on the old mapping, so a property added to the mapping
  reached the store only for records that changed later. The state file remembers each record's
  mapping (`fields_hash`); a record applied by 0.8.0 has none and is applied again once, with no
  model call.
- A web page whose title, description or publication date changed while its URL and body stayed
  the same is a changed document, extracted again once `refresh_after_days` have passed, so its
  `WebPage` node gets the new values. 0.8.0 counted it unchanged. A page applied by 0.8.0 takes
  the fields of its next fetch as applied and is not extracted again for them.
- `cortex run --fetch-only` names in the new `skipped` list each document a run would leave out
  after it failed alone on 3 consecutive runs, and counts it neither `new` nor `changed`.
  0.8.0 forecast it as model work that the run then skipped.
- `create`, `update` and `adopt` read every field the published JSON Schema types `integer` as
  that schema does: a number with no fractional part in any spelling (`timeout_s: 1.0`, `1e3`) is
  that whole number, where 0.8.0 refused `1.0` as not a whole number. A fraction (`1.5`) is still
  refused, naming the field (review of 0.7.2, finding 25).
- The conformance suite reads the spec document each create, adopt and update step supplies
  through the command line's own loader (`load_given`) and gives the behaviour the spec it
  loaded, where it gave every step one fixed spec; a document the loader refuses fails its
  scenario. Three authored scenarios (`spec/scenarios/`) create, adopt and update from
  `cortex.instance/1` documents the loader accepts, and the `spec` input of `CreateInstance`,
  `AdoptInstance` and `UpdateInstance` carries such a document as its `example:`, so every one of
  the 149 steps that supply a spec supplies one the loader reads; none is given a stand-in spec
  (review of 0.7.2, finding 26).
- The specification is validated and generated by ESS 0.98.0 (was 0.77.0), locally and in CI, at
  format `ess/33` (was `ess/22`). ESS 0.93.0 refuses a command group named like a first-level
  command, so `cortex.instance.ProposeSchemaChanges` is listed in the `schema` group of the
  command tree; `cortex schema <name>` and its subcommands are unchanged.
- `cortex update` takes a source the new spec drops out of the registry, and stops and deletes
  its timer and service (`removed_sources`). Before, the source stayed `Enabled` and its timer
  kept firing a run that failed and was counted. A timer that does not stop keeps its source and
  unit files, `units.failed` names it, and the next update tries again.
- `cortex update` gives a source the spec keeps its new `kind` and `schedule` in the registry, so
  `cortex sources` lists the schedule its timer runs on; its runs, failures and state are kept.
- `cortex update` with another `serve.view_port` moves the viewer: `meta.json` and `cortex list`
  hold the new port, and the viewer unit is written on it and restarted. Before, the old port
  stayed. A viewer unit an update rewrites for a changed connection is restarted too.
- `cortex source add` for a source the registry holds answers `source-exists` (exit 1) and
  changes nothing; before, it reset the source's counters and enabled a disabled source. A source
  of an instance the home does not hold is refused (`no-such-instance`), and one of a removed
  instance (`not-active`).
- `cortex remove` answers `stop-failed` (exit 1) and names each failure when
  `systemctl disable --now` fails for a timer or the viewer. It deletes no unit file and keeps the
  instance active, so `cortex remove` can be run again. Before, it answered `removed`, deleted the
  unit files and refused a second remove as `wrong-state`. A unit file that is not there is no
  longer stopped, and `daemon-reload` runs only once a file was deleted.

### Fixed: answers and evaluations (review of 0.7.2, findings 19 to 24)

- `cortex eval run --runner ask` asks the instance's running `cortex ask serve` at the address its
  spec's `ask.listen` names, instead of answering `runner-failed` "no service".
- A Chat Completions refusal (no content, a non-empty `message.refusal`) ends the answer with an
  `error` event whose reason is `refused` and whose message is the refusal text, streams no text,
  and is recorded with the outcome `refused`, the text as its `answer` and a `failed` review item.
  It was a `done` event with an empty answer, recorded as `answered`. Breaking: a new
  `cortex.ask.AnswerOutcome` variant and `cortex.ask.Ask` outcome.
- A citation marker naming an id no tool result of the answer held is written
  `[unsupported node:<id>]`, never linked, and listed in the new `done.unsupported`, not in
  `done.nodes` or `done.evidence`. Breaking: `cortex.ask.AskDone` has the field `unsupported`.
- `replay` keeps a recorded non-answer (a refused case) failed, whatever its text holds; each
  `results.jsonl` line of a case with no answer names it in `outcome`, and its scores count every
  expectation missed.
- The quality judge is shown each fact's validity interval (`Valid: <from> to <to>`), so one fact
  valid in January and in February is judged at its own time.
- An answer ended by the step or time limit, a model failure or a reader failure after a model
  call keeps the steps and tokens of the calls made, in its error and in its record; they were 0.
  Breaking: `cortex.ask.LimitReached`, `ModelFailed` and `ReaderFailed` carry `steps` and the
  token counts.

## 0.8.0 — 2026-11-23

### Added

- `cortex run <instance>/<source> --path <dir>`, also with `--fetch-only`, reads a `files` source
  from `<dir>` instead of its spec's `paths`, for that run only. Each document is keyed by its
  path relative to `<dir>` (`<relative path>#<id>`, evidence `file:<key>`), so a record read from
  a new directory on each run is one document and is not extracted again while it is unchanged.
  On a source of another kind, or on `<instance>/seed`, it answers the new outcome `path-refused`
  (exit 1) and writes nothing; a missing `<dir>` fails the fetch, naming it.

### What 1.0 holds

1.0 is the first release whose interface is stable. It adds no feature over 0.5.0; it is 0.5.0's
feature set after a real instance ran for 7 days with nobody at the keyboard.

- **Instances.** One spec file makes an instance: an EKR store on SQLite or PostgreSQL, a viewer,
  an MCP line, and one systemd user timer per source. `cortex update` changes sources, model and
  serve settings; `cortex adopt` turns an existing store into an instance without reseeding;
  `cortex remove` keeps the data.
- **Sources.** Web (search, or listed sites read as pages or crawled), Connectors (records of any
  admitted operation, with pages, child calls and a time window), files (local files or records
  read from them, with lookups), and structured imports that need no model call, map child
  operations as linked nodes, link changes to the first tag that ships them, and end what a source
  dropped.
- **Runs.** Only new or changed documents are extracted, by a model call with no tools. cortex
  mints every evidence id, dates facts by when their source said them, supersedes a changed value,
  and writes extracted relations as edges. Credentials are masked before storage; a `redaction`
  policy keeps personal data from the model and a gate refuses a batch it still finds. A snapshot
  before each run makes `cortex restore` undo a bad one, and a gated run on PostgreSQL runs in an EKR
  stage it abandons on failure; two failed runs in a row disable a source, and a daily ceiling
  holds model spend. A metric file, a failure hook and `cortex sources` show how each run ended.
  `cortex quality` judges a sample of facts; `cortex schema` proposes schema changes.
- **Project.** Specified in ESS with 132 conformance scenarios; CI runs `task check` on every pull
  request; each release carries an `x86_64-unknown-linux-gnu` binary checked against `SHA256SUMS`.

Open limits in 1.0 ([Limits](https://beyond10x.github.io/cortex/docs/limits)):

- `model.backend: Codex` is accepted and fails every run; extraction uses `claude -p` only.
- A web page is cited as a `HumanStatement` naming its URL, until EKR admits URL evidence.
- Linux with systemd user units only, and one released target.

### Added

- `cortex run --no-wait`: while another cortex command holds the home's lock, the run answers
  `busy` at once with exit status 9 and runs and records nothing, so an external scheduler can
  skip a collection that is still busy. Without the flag a run waits for the lock, as before
  (review of 0.7.2, finding 18).

### Changed

- Development only, no change to the binary: the integration tests are one test binary
  (`tests/main.rs`, a module per file under `tests/it/`) run by cargo-nextest, which kills a test
  over 60 s and prints the step's wall time and ten slowest tests (`TEST_BUDGET=enforce` fails the
  step over 120 s); test builds carry no debug info; `rust-toolchain.toml` pins rustc for the gate
  and every workflow; and a pull request that changes only planning or prose files passes `Check`
  without the build and the tests.

### Fixed

- A structured source with `dropped: Supersede` retracts a record's values only when a whole
  listing no longer lists it. A Connectors operation asked about the run's window (`{since}` or
  another window placeholder in an input, or `stop_older_than_since`) supersedes the changed
  values of the records it lists and keeps every value of the records it does not list; 0.7.2
  retracted them.
- A paged answer that names a page or token the walk already asked for is a paging cycle: the walk
  leaves the operation's pages unread, fetch-only lists it in `unread`, a run names it in
  `stopped`, and nothing is ended. 0.7.2 read the cycle as the end of the listing and retracted
  the records it never reached.
- A record a whole listing dropped is retired with its values: when the source lists it again,
  with the same text, it is new and applied again. 0.7.2 called it unchanged and its values stayed
  retracted.
- A structured record whose entity EKR reports `ambiguous` (its aliases match two store nodes of
  its type) is named in `rejected`, not counted in `documents_applied`, not marked seen, and tried
  again by the next run. 0.7.2 counted it applied and never tried it again.
- A refused structured record (rejected, ambiguous or refused by `store_lookup`) is tried after
  every other record the run wants, the one refused longest ago first, so under
  `max_documents_per_run` it no longer blocks the records behind it on every run.
- The EKR pin an instance records is the engine that creates its store. `cortex create` and
  `cortex adopt` ask the `ekr` binary for its version (`ekr --version`) and refuse one that
  reports another version than the spec's `ekr.version`, whether `ekr.bin` names it or it sits in
  the cache; `cortex setup` refuses a cached binary of another version instead of keeping it
  (review of 0.7.2, finding 17).
- A connection named by id is found on any page of `connectors connections list`: cortex follows
  `next_cursor` when `cortex create` checks the id and when it revalidates the connection before
  a read. Before, an id on the second page was reported missing, and two timer runs then disabled
  the source. A list that names a cursor a second time, or runs past 1000 pages, ends the read
  with a reason naming the cursor instead of a loop.
- `RareName` no longer replaces the store's ontology names (node types, properties, value kinds,
  relations) in the prompt or in a document, and does not count them. A model writes back the
  type names it is shown; a replaced one made EKR refuse every entity
  (`extraction-type-undeclared: Name-32`) and the run was undone. `known_names` and `rules` still
  replace a type name they match.
- A placeholder the model writes where a node type, property or relation name goes is restored
  before the answer is applied, also without its brackets (`Name-3`); one that stands for no value
  fails the batch, naming it and where it stood.
- A batch directory under `runs/` keeps the prompt as sent (`prompt.txt`) and the model's raw
  answer (`answer.json`, formerly `model.json`) under a `redaction` policy too. Both hold
  placeholders, not originals; the mapping is never written.
- `cortex create --no-units` and `cortex adopt --no-units` print `view: null` instead of an address
  no viewer serves, and `cortex list` prints `view: null` for an instance with no viewer unit.
- **One current value per single-valued property.** A `!Property` fact of a property declared
  `cardinality: One` (by the store, or by the model's document for a new one) is marked
  `replaces: true` by cortex before it is applied. EKR 0.0.34 adds an unmarked fact as one more
  active value whatever the cardinality, so a merge request's state read from its tracker record
  and again from a later chat message stayed current twice. Now the newest value stays current,
  an older one is kept as its history (below), and a first value is stored
  through `facts_reapplied` (`story:extraction-coverage`).
- **The prompt asks for every document's own facts.** The extraction rules tell the model to go
  through every document to its end, to extract a thread reply's own facts apart from the message
  it answers and every item of a message that lists several, and to read `[context]` paragraphs
  as context (`story:extraction-coverage`).
- A document read after a newer one no longer loses its changed value: a fact marked
  `replaces: true` that is older than the current value, which EKR refuses
  (`invalid-supersession`), is stored as that value's history, ending where the current value
  begins, and counted in the run's `facts_history` instead of `rejected`. Before, the fact was
  lost and listed as rejected.
- One document whose extraction redeclares a type the store holds otherwise
  (`extraction-type-conflict`) no longer fails its whole batch: that document is refused alone,
  named in `rejected` with `extraction-type-conflict: <type>` and not marked seen, so a later run
  asks for it again, and the rest of the batch applies. Before, the batch's apply failed, which
  failed the run when nothing had been applied yet and stopped it otherwise.

## 0.7.2 — 2026-10-30

### Fixed

- Now masked on every run, and found by the `Credential` class: a password under a name glued to
  a prefix (`PGPASSWORD=`) or cut short after one in upper case (`DB_PASS=`, `DB_PASS: `,
  `"DB_PASS":`), a secret followed by `key` (`SECRET_KEY=`, `DJANGO_SECRET_KEY=`,
  `AWS_SECRET_KEY=`, `secret_key = `), a name in Markdown or chat markup (`` `DB_PASSWORD`: ``,
  `*AWS_SECRET_ACCESS_KEY*: `, `**api_key**: `, `**Password:** `), a name with a leading `_`
  (`self._password = `), an XML element with or without a namespace, its text or a CDATA section (`<db_password>…</db_password>`,
  `<wsse:Password …>…</wsse:Password>`), and a command-line flag with its value after a space,
  a quoted value whole, also escaped in a JSON string or after `=` (`--db-password "…"`).
- Kept as they are: `access_key_id`, `secret_santa`, `password_policy`, `--password-file`,
  `<password_hint>`, a `<password>` placeholder not closed by its own tag, a heading or label
  with no value, `PGPASSFILE`, `pass` alone, a lower-case `_pass` name (`render_pass`,
  `season_pass=`), and after a flag a word of prose in any script or capitalised (`option`,
  `ändern`, `Option`), a metavar also in brackets (`PASSWORD`, `[--password PASSWORD]`,
  `<DB_PASSWORD>`, `(string)`), a variable also quoted (`$API_KEY`, `"$PGPASSWORD"`) or a
  `--secret id=…,src=…` mount.
- A name in one of these spellings followed by a bare `:` and its value (`get_secret_key:P-1`)
  is read only in the spellings 0.7.1 read, so no structured record identity changes.
- The `Credential` class also reads a JSON-quoted name inside a JSON string
  (`{\"db_password\": \"…\"}`) and a value that starts with a percent-encoded byte
  (`aws_secret_access_key=%2B…`), as masking does.
- A URL password after a user name that holds a raw `@` is masked when it holds a `,` or `;`
  too; a user, host and port before a list of addresses (`ops@mail.example:587,admin@example.org`)
  is still kept.
- The two cases 0.7.1 listed under Known limits for `story:credential-key-spellings` run and
  pass (`tests/w30b_mask_adversary.rs`).

### Known limits

- A password of lower-case letters only after a flag and a space (`--password hunter`) is read as
  prose and kept.
- A new spelling followed by a bare `:` and its value (`SECRET_KEY:v`) is not masked, so that no
  structured record identity changes.
- `pwd`, `passphrase` and `token` are masked only by the `Credential` class, not on every run;
  a single-dash flag (`-p v`) and an HTML-escaped element (`&lt;password&gt;`) are not masked.

## 0.7.1 — 2026-10-30

### Fixed

- Across two homes, an instance whose unit name is another instance's (instance `a-b` with source
  `c` and instance `a` with source `b-c` both name `cortex-a-b-c`) took over the other home's live
  unit, and that home's source stopped running. `create`, `update` and `adopt` now refuse before
  changing anything, naming the unit, the home that holds it and its source or viewer there. A unit
  whose home no longer holds the instance it belongs to is still taken over (`units.taken_over`).
- In a URL or record key, a raw `%` before an encoded `&` or `#` (`<token>%%26page=2`) no longer
  carries the rest of the query into the masked value, so two URLs that differ after it stay two
  documents. A twice-encoded `&` or `#` (`%2526`) ends a masked value too, and a `%` before
  another escape or character (`%25%21`, `%%41`, `%2%41`, `%2%x`) stays in it.
- Upgrading: a key 0.7.0 stored for a URL with `%%26` in its query
  (`access_token=[masked:secret-assignment]`, the rest of the query inside the masked value)
  matches no key after the change. Each such document is read once more after the upgrade,
  under its new key, and the old key stays in the state file, matched by nothing.
- Now masked: a password assigned with a twice percent-encoded name or separator
  (`%253Faccess_token%253D<token>`), a JSON-quoted password inside a JSON string
  (`{\"password\": \"<token>\"}`), and a URL password after a user name that holds a raw `@`
  (`ftp://jane@corp.example:<password>@host`) when the password holds no `,` or `;`, so a user,
  host and port before a list of addresses (`ops@mail.example:587,admin@example.org`) is kept.
- Now masked without a redaction policy: a value assigned to an access key by name
  (`AWS_SECRET_ACCESS_KEY=<v>`, `aws_secret_access_key = <v>`, `access-key: <v>`). 0.7.0 left
  it in place unless the policy held the `Credential` class.
- Masking a key that was masked already no longer masks it again when a `%3A`-separated
  assignment follows a scheme (`x://password%3a<token>&u@h`); such a stored key is no longer
  dropped on every load.
- A client of `cortex ask serve` that sent a body above a route's limit (1 MiB for `/ask`, 64 KiB
  for a vote or a close) could lose the 413 refusal: the service answered and closed the
  connection with the body unread, which reset it while the client was still writing. The service
  now reads the rest of a refused body, up to 16 MiB and for at most 10 seconds, before it
  answers; so it does for an unknown path (404), a wrong method (405) and `GET /review`. The
  eval `mcp` runner's redacting server likewise reads up to 16 MiB of the declared body, for at
  most 10 seconds, before its 401, 404, 405 and 413 refusals, and answers a client that sent
  `expect: 100-continue` at once.

### Known limits

Each has a test kept `#[ignore]`d that names `story:ignored-adversary-limits`.

- `cortex update` does not refuse a redaction rule that rewrites the ids of a `structured` source
  with seen records, though the rewrite re-keys them (`tests/rekey_adversary_pass2.rs`).
- A record member literally named `$` and the whole record id `$` are one key path
  (`tests/rekey_adversary_pass2.rs`).
- A run that failed before it applied a batch does not count the tokens its failed answers
  reported ([Limits](https://beyond10x.github.io/cortex/docs/limits),
  `tests/partial_counts_adversary.rs`).
- A credential that ends in a raw `%` and one hex digit right before an encoded or raw `&` or `#`
  (`<token>%2%26`, `<token>%2&`) keeps that `%` and digit in clear in a key
  (`tests/w30b_mask_adversary.rs`).
- In a URL with a user name and a port, a `)` and an address right after the port
  (`(smtp://ops@mail.example:587)admin@example.org`) are masked as a password
  (`tests/w30b_mask_adversary.rs`).
- The eval `mcp` runner's redacting server drains only a body declared by `content-length`, so a
  client that sends a chunked body before it reads may lose the server's refusal
  (`tests/w30b_ask_adversary.rs`).

Kept `#[ignore]`d under `story:credential-key-spellings`, for the next release:

- A value assigned to `secret_key`, `SECRET_KEY` or another name with `secret` before its last
  word is kept in clear, with or without a `Credential` policy (`tests/w30b_mask_adversary.rs`).
- A URL password holding `,` or `;` after a user name with a raw `@` is kept in clear
  (`tests/w30b_mask_adversary.rs`).

Kept `#[ignore]`d under `story:name-redaction-boundaries`, since 0.6.1
(`tests/redaction_beside_marks_adv.rs`):

- A known name glued to CJK text or an underscore, a name followed by a footnote digit, a name in
  a lower-case-prefixed file name and a name starting with a titlecase digraph are shown to the
  model unreplaced.

## 0.7.0 — 2026-10-30

0.7.0 needs Connectors 0.60.0 or newer: a Connectors source refuses an older one, naming the version.

### Added

- An input of a `connectors` or `structured` source may use `{since_minutes_ago}`: the whole
  minutes from the window's start to the time of each call (each input, each page, each retry
  after a renewal), rounded up (`updated >= -{since_minutes_ago}m`), for a query language that
  takes relative dates rather than RFC 3339.
- A spec may give `runs: {keep_days: <n>}` (at least 1). At the end of each run, the run
  directories under `runs/` older than that are removed; the current one, and any other name or
  file, are kept. `runs/` belongs to cortex: a directory there named like a run directory is
  removed by the same rule, whoever made it. Without it nothing is removed, as before, and `runs/` grows with every batch.
- A `structured` source's child may carry a `follow` rule (`equals`, `within_days` or
  `at_least` on a field of the parent record), and the run calls that child only for the parents
  the rule matches. A parent it does not match is still applied as a node; its children of that
  operation are not asked for, and none of its earlier ones is ended. `equals` and `at_least`
  compare whole numbers exactly, also above 2^53. The run's `skipped` counts
  those parents per child operation. A child without `follow` is called for every parent, as
  before.
- A `connectors` source may give a `key` template (`{input.channel}:{ts}`) that a document's key
  is rendered from instead of `id`, with `{input.…}` naming the input the record was read with, so
  two records with one id in two inputs are both applied; before, the second was dropped.
  `cortex update` refuses a change of `key` while the source has seen documents (`rekey-refused`),
  including adding `key` where none was, and, while `key` names an `{input.…}`, a respelled input
  (`channel: C1` becoming `channel: general`); adding, removing or reordering inputs, or changing
  an input field the key does not name, is accepted. A `key` that is not a template (empty,
  without a placeholder, or with an unclosed `{` or a stray `}`) is refused when the spec is read,
  as an alias template is, and so is an `{input.…}` placeholder that some input does not hold as
  a text, number or boolean, or one on a source without inputs.
- A `connectors` source's `child` may give `text` templates, and each child record is then added
  to the parent's text as its rendered lines instead of raw JSON, so more children fit under
  `max_chars_per_document`. It may also carry a `follow` rule, the one `structured` children take
  (`{rule: at_least, value: {field: reply_count, number: 1}}`), and is then called only for the
  parents it matches; another parent's text is its own. A `child` without either behaves as
  before. Adopting `text` changes every parent's text, so each is extracted once more.

### Changed

- The specification is validated and generated by ESS 0.77.0 (was 0.60.0), locally and in the
  `Check` workflow. Its conformance suite has 124 scenarios.
- The specification states changes this release makes: `cortex.instance.SourceRan` carries
  `facts_reapplied`, as `cortex run` prints it, and `cortex.ask.CastVote` and
  `cortex.ask.CloseReview` name the record routes' `bad-request` (413 above the routes' new
  64 KiB limit, else 400) and `store-failed` (500, now a fixed message and a `request_id`) answers
  as `cortex.ask.BadRequest` and `cortex.ask.StoreFailed`. The reference pages under
  `docs/reference/ess` show them.
- `task check` runs `task compat`, which fails when the specification changes in a breaking or
  unknown way against the newest release tag and `spec/compat-ack.json` does not acknowledge the
  change; each acknowledged change has its reason in `spec/compat-ack.md`.

### Fixed

- A `connectors` call that hung held its run until something outside ended it. Every call is now
  waited for at most `CORTEX_CONNECTORS_TIMEOUT_S` seconds (default 120); past it the call is
  killed and fails as `timeout`, which fails the run as `fetch-failed`. The bound holds for the
  whole call: a process the call left holding its output is killed at the deadline with the
  call's process group, and so it is at once when the run is sent SIGTERM. A setting too large for
  the clock sets no bound instead of panicking.
- A `files` source read every file whole, however large, and left out a file it could not read
  or that was not UTF-8 text without a word. A file over 16 MiB, unreadable or not UTF-8 text is
  now named in the run's `skipped` with the reason. A `structured` source's file over 16 MiB fails
  the fetch, naming the file, as an unreadable one already did, so `dropped: Supersede` never ends
  the values of a file it did not read. A `records` lookup file over 16 MiB fails the fetch too, and so does one that is not a regular file (a FIFO
  blocked the run).
- A model document holding an object key `"!"` (a tag with no name) panicked the run; it is now
  an extraction error naming where the key is, and nothing of that batch is applied.
- What `ekr` wrote to stderr when a store verb failed reached the run's report, `cortex.log` and
  the source's status unmasked; it is now masked for credential shapes, and an unknown
  publication outcome still reads as `store-outcome-unknown`. An `ekr session`'s stderr is masked
  before it is cut to its last 16 KiB, so a cut cannot part a credential from its name. Lines
  dropped from the 1 MiB it holds stay as the context the kept ones are masked in, so a private
  key whose `BEGIN` line was dropped is still masked. The same holds for `cortex quality`, the
  `schema apply` gate and `adopt`; the ask service's `postgres` answer records now log their
  launch's stderr through the masked log instead of inheriting it, and a monitoring hook's stderr
  is masked before it is cut.
- A store `ekr` cortex starts itself (no Connectors launch) inherited every `EKR_*` variable of
  cortex's environment but `EKR_STAGE`, such as `EKR_FULL_REPLAY` from the operator's shell; it
  now gets only the ones cortex sets, as a launched one already did. So does the ask service's
  MCP reader.
- An `ekr example ekr.cli-host/1` answer whose host, `authority` or `validation_profile` is not
  an object panicked `create` and `adopt`; it is now an error.
- Two `structured` sources whose records share one identity prefix (the same adapter and
  operation, or the same paths and glob) were accepted with `dropped: Supersede`, and a run of one
  ended the values of records only the other listed. `create` and `update` now refuse the pair,
  naming both.
- A `connectors` record whose id was JSON null was keyed `<adapter>:<operation>:null`, so all
  such records shared one document and all but one were dropped without a word; a record without
  an id was dropped silently. Both are now left out and counted in the run's `skipped` with the
  reason. A source whose seen state holds a `…:null` key no longer refreshes that document.
- A document the model failed on every time stopped its source at the same batch on every run,
  and nothing counted failures per document. A batch of several documents the model answers it
  failed on still fails the run as before, after one call, and now counts one failed batch for
  each of its documents. A later run asks that batch whole once more, so one failed answer costs
  no call per document; when it fails again, each of its documents is asked alone, those that
  failed alone fewer times first, in that run and after, so a failing call that spends the run's
  budget still leaves the next run a document further. A document that
  fails alone is counted in `failures/<source>.json`, beside `state/`, so a run its gate undoes
  keeps the count; from its third consecutive failure it is skipped while its text is the one
  that last failed, named in `skipped` with its count and the reason, not sent to the model, and
  the batches after it run. A changed text keeps the count and is asked once more alone. Only an
  answer marks or counts: an error answer that cost nothing (a sign-in or a usage limit) and a
  call that timed out, crashed or left no JSON answer do not. A run that read the source's whole
  listing drops the counts of documents it no longer lists.
- Under a backlog, a source's window stayed at its start for good: one document that kept failing
  held every later run's `{since}` there, also before the first successful run. A run that does
  not succeed now moves `pending_since` to the earliest record time of a wanted document it did
  not apply, less 5 minutes, when it read every record and each such time is readable, and the
  next run asks from there; a failed run follows the same rule. A run that left records unread or
  a document without a readable `time` keeps its `{since}` as before, and a document skipped
  after failing alone holds no window. The window never moves past a change the run held back
  by `refresh_after_days`, also when the run fails.
- `cortex update` accepted a spec that changed how a source keys its documents, so the source's
  seen state matched nothing and its next run extracted every document again. An update that
  changes a source's `kind`, a `connectors` source's `adapter`, `operation` or `id`, a `files`
  source's `records` format (none and `WholeFile` are one) or, per record, its `records.id`, a
  `files` root that reaches a seen file under another spelling (`docs` to `./docs`; adding,
  removing or reordering roots is not refused), or a `structured` source's `input`, `adapter`,
  `operation`, files `paths` or `glob` as written, or `mapping.id`, while `state/<source>.json`
  holds a seen document, is now refused as `rekey-refused`, exit 1, naming the source and the
  field; nothing is written. Record paths are compared as they are read, so `key` and `$.key`
  are one. A
  source with no seen document is updated as before. To re-key on purpose, remove the source's
  state file first.
- `cortex schema` recorded a proposal whose schema transaction EKR could not say took effect
  (`publication outcome is unknown`) as `refused` and went on to apply the next ones against an
  ontology that might hold it. The round now stops there as the new `store-outcome-unknown`
  outcome, exit 4 as a run's, naming the proposal: no later proposal is asked of EKR, and the ones
  before it stay recorded. `cortex schema apply` answers its own `store-outcome-unknown` with exit
  4 too, instead of 1.
- `cortex schema apply` answered `apply-failed` with "every approved proposal is still approved"
  when the store's head could not be read after the stage was published, and did not write the
  decisions. It now writes them, and the reason says the changes are in the store and names the
  revision: the head the stage was published at on `postgres`, the last committed one on
  `sqlite`; when no proposal was applied, it says no change was committed. When
  `proposals.jsonl` cannot be rewritten after the publish, the reason says the file still lists
  the proposals `approved`.
- `cortex schema` answered `propose-failed`, whose summary said nothing was applied, when a line of
  `proposals.jsonl` could not be written after earlier proposals were applied. Its reason now
  names the proposal, how many proposals are applied and how many lines are written, and the
  summary says so.
- `cortex ask serve` with PostgreSQL answer records waited without bound on a records process
  whose database stalled, holding every later ask, vote and review behind it until a restart.
  Record calls are now served in the order they arrive, and a call waits at most 30 seconds for
  the calls before it, for a start of the records process and for the answer to its statement;
  the records process runs each statement with a `statement_timeout` of 30 seconds. A call past
  one of them answers `store-failed`. A process that did not answer in time is stopped and the
  next call starts another; a call that only waited behind others leaves the process alone, and a
  start that fails fails the calls that waited through it instead of each starting another.
- The redacting MCP server of the eval `mcp` runner kept every connection it accepted open until
  the case ended, so a client making many short requests held one descriptor per request, and at
  the descriptor limit the server stopped accepting. A connection now leaves its open list once it
  is answered. It also read request and header lines without a cap: the head is now read up to 64
  KiB, before the bearer check, and a line that reaches the cap is answered 431 with nothing more
  read. Its documentation no longer says it cuts tool results as the ask service does; it passes
  each redacted result whole.
- A run sent SIGTERM during its fetch ran the fetch out before it stopped. The fetch is now a safe
  point: the `connectors` call in flight is ended with its process group (SIGKILL after 5
  seconds), a paged or `files` read stops between pages and files, and the run ends `terminated`,
  exit 8, with nothing marked seen and its window kept. A run stopped in the wait before a model
  call is asked again says so in its reason, instead of saying a call was ended.
- A Connectors source failed every run against Connectors 0.60.0, which removed `operations
  invoke`. cortex now calls `connectors invoke <connector>/<instance> <operation> --connection
  <id> --input-stdin`, with the connector and instance read from the connection's `connections
  list` entry, which carries them from Connectors 0.54.0 on, and needs Connectors 0.60.0 or newer.
  An older one fails the run naming that version: from 0.54.0 to 0.59.x by its parse refusal of
  the `invoke` command line, before 0.54.0 by the list entry without a connector and instance,
  naming the connection. A failure envelope that carries its code only in `error.code`, as a
  parse refusal does, is now named by that code instead of "unknown at unknown". The input goes
  on standard input, so no record field or cursor is in the process list, and each call is one
  `invoke`, without a description read before it. A `result` that is not a JSON object fails the
  call naming its JSON type.
- A model call that failed or was cut off at the output limit counted in the day's spend but not
  against the run's `budget_usd` or in the run's `cost_usd`, so a split batch was given more budget
  than remained and the run reported, and a gate's `cost_usd` check read, less than it spent. Every
  costed answer now counts in all three. A run with a failed call that carried no cost now reports
  `cost_usd` null, as `cortex quality`, `cortex schema` and `cortex eval` do.
- A model call that had already finished when the run was sent SIGTERM was dropped as ended, and
  what it cost counted nowhere. Its answer is now read: its cost and tokens count in the day's
  spend and the run's, and the run still ends `terminated`.
- A fact the model marked `replaces: true` with no current value to replace was rejected by EKR
  (`replacement-without-active-assertion`) and its value lost, its document marked seen. cortex now
  applies such a fact again in the same batch without `replaces`, so the value is stored as a plain
  assertion; it is not counted in `parts_rejected` or listed in `rejected`, and the run reports it
  in `facts_reapplied` (in the `ran` detail, and in the `cortex.log` line when not 0). Several
  such facts of one entity and property are applied in their documents' time order, the later
  ones still replacing, so one value stays current. A plain value of the same entity and property
  in the batch, which EKR stored, takes its place in that order: an older replacement is not
  applied again and stays in `rejected`, and a newer one replaces it, so the batch does not leave
  two values current. A re-apply the store fails stops the run and
  leaves the documents it carried unseen, so a later run extracts them again.
- `cortex ask serve` recorded an answer whose reader closed the stream as `model-failed`, with no
  tokens, and opened a review item for it. It is now recorded with the outcome `client-closed`,
  the token counts and steps of the model calls already made, and no review item, `slow`
  included. A reader who leaves while the model is asked does not cut the call, and no tool call
  of that reply is made.
- `cortex ask serve` streamed and recorded a model reply cut off at its token limit or by a
  content filter as an answer. Such a reply (`finish_reason` `length` or `content_filter`) now
  ends the stream with an `error` event, reason `cut-off`, is recorded with the outcome `cut-off`
  and its tokens, and opens a `failed` review item. An older cortex cannot read a record with
  either new outcome.
- `cortex ask serve` had no bound on answers in flight, tool calls per step, the gateway's
  answer, or the bodies of votes and review closes. `ask.max_in_flight` (default 8) caps the
  answers worked on at once: a question past it answers 429 `busy` at once and spends no question
  of the minute. An answer keeps its slot until its reader is stopped and its model call has
  ended, also after its reader left, so clients who disconnect cannot run more readers or model
  calls than the cap. `ask.limits.max_tool_calls` (default 16) caps the tool calls one step makes; the
  calls past it are not made, the model is told so, and a `status` event says how many. A gateway
  answer above 4 MiB fails the answer with `model-failed`, and a vote or close body above 64 KiB
  answers 413. The redaction policy is compiled once when the service starts, not twice per
  question. Readers stay one per question, killed when the answer ends.
- `cortex ask serve` sent internal text to HTTP clients: the database path and SQL error of a
  record route's 500, and the gateway's answer or the reader's program path in an `error` event.
  A record route's failure now answers `store-failed` with a fixed message and a `request_id`;
  an answer's `error` event carries a fixed message for `model-failed` and `reader-failed`, ending
  `(request <answer_id>)`. The detail goes to standard error on a line naming the same id,
  masked for credential shapes like every line the service logs.
- A run reads the store once (`ekr snapshot` at its first batch) instead of once per batch, and
  twice per batch under a `refuse_if_left` policy. The nodes each batch applies, those its
  re-applied replacements create included, are added to what it read, so the next batch is still
  shown them as in the store; a commit another writer makes
  during the run is seen by the next run. On a large store each snapshot took minutes. Batch 0 is
  no longer pseudonymised twice under `refuse_if_left`, and the store's ontology is read again
  only after an apply.
- A run started one `ekr` process for every store read: the extraction schema, the ontology before
  each batch, each snapshot and the gate's quality check, and on a store reached through Connectors
  each was one `connections launch` with a `connections list` readiness check before it. A run now
  opens one `ekr session` after it begins its stage and closes it before it publishes or abandons
  the stage, and every read goes through it. `ekr apply-extraction` and the stage verbs stay one
  process each, as EKR refuses them in a session; the session sees each batch's apply at its next
  read. A launched store's connection is checked (`connections list`) once per run instead of
  before every launch, and again only when a launch is refused because the connection's validation
  evidence lapsed during the run, which is then made once more. The snapshot is read from the
  session's output into the fields cortex uses, skipping the rest as it is read, instead of being
  held whole as a JSON document. A run sent SIGTERM while it waits for its session to answer a read
  ends the session and stops `terminated`, undone when gated; a write in flight is waited for. A
  stop of the run's unit (`systemctl stop`), whose SIGTERM ends the session too, ends the run the
  same way: a staged run abandons its stage instead of publishing what it applied. A
  store that does not open when a run starts still fails it as `apply-refused`, not
  `extraction-failed`.
- A `dropped: Supersede` source whose run selects nothing, and whose listing, held-back records
  and failed child calls are those of the last run that ended its dropped values, no longer
  begins an EKR stage or reads the store (`ekr snapshot`): such a run read the whole store to
  end nothing. The state file remembers that listing's fingerprint (`ended_listing`). Comparing
  the store with a listing no longer scans every assertion of the source for each value it ends.
- The `redaction` policy panicked a run on a link without a scheme whose host has a multi-byte
  character in its first four bytes (`café.fr/menu`), let the rest of a number written
  `+49 30/1234567` through, and ended a link at `[`, so `?filter[owner]=x` reached the model.
  Such a link is now replaced, and an international number may have one `/` after its area code
  (a date after `+`, such as `+1 2/3/2024` or `+1 2024/10/30`, is still not a number). A link
  keeps the brackets and parentheses it opens and closes, and a `(` or `)` without a partner
  while the link goes on after it (`?q=Jane(Doe`, `/a1)report.pdf`). It ends at a `]` it did not
  open, and at a `)` it did not open before a space, a bracket, a quote, the end of the text or
  the punctuation that closes a sentence, so `[https://host/x]` and Markdown links written back
  to back without a space are each their own link. A citation such as `[2]` after the
  punctuation that ends a link stays outside it.
- The always-on credential mask missed five shapes: a quoted name (`"password": "…"`), a URL
  password with a raw `@` (`postgres://app:p@ss@db/x`), a value in a document key with a raw `%`
  that starts no escape, `%3A` as the separator, and a private key whose `END` line a provider
  cut off. Each is now masked; a separator may also be `=>` or follow a percent-encoded quote, and
  a cut private key is masked through its last base64 line. A URL password ends before a
  character a URL never holds raw, such as the `"` that closes a JSON string. A document whose
  stored key holds one of these shapes, or the rest of a credential an earlier version left after
  its `[masked:…]` token, is read again once under its masked key; the old key, and a
  child-failure count kept under it, is dropped from the source's state.
- Two instances of one home could name one systemd unit, because a name may hold `-`: instance
  `a` with source `b-c` and instance `a-b` with source `c` both wrote `cortex-a-b-c`, and a
  source `b-view` of `a` wrote the viewer unit of `a-b`. The second install overwrote the first,
  and `remove` could delete the other instance's unit. `create`, `adopt` and `update` now refuse,
  before changing anything and with or without `--no-units`, a spec whose source or viewer unit is
  another active instance's of the home, or a source named `view`, naming the unit and both
  owners. Unit names do not change.
- `cortex update` enabled and started the timer of every source, so a source two failures had
  disabled ran again on its schedule while `cortex sources` still showed it disabled. `update`,
  `create` and `adopt` now write a disabled source's units without enabling its timer;
  `cortex source enable` turns it on again.
- A spec number out of range was accepted and failed later: a large `refresh_after_days`
  overflowed when a run counted its window, a hook `timeout_s` near the largest integer panicked
  the run after its line was printed, and a `view_port` of 70000 or -1 wrapped to another port.
  `create`, `update` and `adopt` now refuse, naming the field, a `refresh_after_days` above
  36500, a `monitoring.hook.timeout_s` above 86400 and a `serve.view_port` outside 0 to 65535.
  An instance created by 0.6.1 with such a value keeps running: its frozen copy
  (`instances/<name>/instance.yaml`) is not held to the bounds, a run counts a larger refresh
  window as reaching back as far as a time can, a larger hook `timeout_s` is capped at 86400
  seconds, and the port is read only when an instance is made. An `update` to an out-of-range
  value is refused naming the field, and one into range is made as before. `cortex ask serve`
  is not held to the bounds either: it reads none of these fields and serves such a spec as
  0.6.1 did.
- A home file cortex rewrote (`registry.json`, a source status, a state file) was renamed into
  place without being synced, so a crash could leave it empty and every command then failed. Each
  is now synced to disk before the rename, and its directory after it.
- A source status file that could not be parsed was read as empty, and the next run rewrote it
  with its counts started again. `cortex sources` now fails, naming the file, and a run leaves it
  as it is and records why in `cortex.log`; other commands name it on standard error and go on.
- A `create` or `adopt` that failed after recording the instance (its meta or a source could not
  be saved) left it registered, so a retry answered `name-taken`. It now takes the instance and
  its sources out of the registry again and removes the instance directory.
- A `restore` whose store was put back but whose copy of `state/` could not be written failed as a
  whole, though the store had changed, and a gated run's undo then said the store still held the
  run. The restore now answers `restored` with `state_restored: false` and the reason in
  `state_failed`, and the undo says the store is back and `state/` is not.

## 0.6.1 — 2026-10-17

### Added

- `metrics.prom` reports what each run of a source did: `cortex_source_last_run_duration_seconds`,
  `cortex_source_last_run_documents{kind="read|applied"}`, `cortex_source_documents_total{kind}`
  and `cortex_source_model_tokens_total{kind="input|cached|output"}`, beside the runs by outcome.
  No series holds a credential, a prompt or document content.
- `cortex run` handles SIGTERM: the run stops at its next safe point (ending a model call in
  flight), a gated run is undone, and it ends `terminated`, exit 8, with its status, metric file
  and hook written; `--record-failure` does not count it. A second SIGTERM exits 143 at once.
- A run whose model cannot be reached (connection refused, `claude` not startable, the model not
  loaded, a gateway answering 502, 503 or 504, an API answering 529 overloaded) ends
  `model-unavailable`, exit 7, with its cause (`refused`, `not-loaded`, `gateway-down`), instead
  of `extraction-failed`; `--record-failure` does not count it, so a model outage never disables
  a source. Batches applied before it stay. A stop that also ends the model call or the
  `connectors` fetch still ends `terminated`, and the wait before a model call is asked again
  ends on SIGTERM.
- Before each batch, a run looks the batch's document keys and record ids up in the store, across
  every node type, and shows the model each node it finds (type, name, aliases, marked as in the
  store) ahead of its own known names, so a node an adoption or another writer stored is reused
  instead of created again under another type. The 50-per-type bound holds for the run's own
  names only. Names match lowercased. A store node's name and aliases are masked for credential
  shapes on every run and written on one line, and behind a model window they count in the
  batch's fixed part.
- A structured mapping's `alias_templates` add fixed-format aliases from record fields
  (`repo:{namespace}/{path}`), so a source's records resolve to an adopted store's nodes of the
  mapping's own node type. A
  template whose field a record lacks yields no alias; another placeholder syntax is refused when
  the spec is read.
- A figure renderer for the ask service (`cortex.ask.FigureSpec`): a bar chart, a timeline or a
  relationship graph drawn as SVG by cortex's own code, from typed values only. The SVG holds only
  allow-listed elements and attributes, with every label as escaped text; the same figure gives
  the same bytes; a figure over a cap (50 bars, 200 timeline entries, 100 nodes, 300 edges,
  256 KiB) is refused whole. No endpoint draws one yet.

### Changed

- The reason of a model call that exited without a JSON answer is masked for credential shapes
  and bounded, as an error answer's is.

### Fixed

- An RFC 3339 instant with a character outside ASCII (`2026-01-01T00:00:0éZ`) made a figure's
  timeline or a structured record's date panic; it is now refused as no instant. `Z` followed by
  an offset (`…00Z01:00`) is refused instead of read as the offset.

## 0.6.0 — 2026-10-17

### Added

- `cortex ask serve --spec <instance spec>` answers questions over an instance's store as a
  `text/event-stream`: a tool loop over the EKR MCP reader (`ekr mcp`) against the spec's
  `model.gateway`, configured by the spec's new `ask` section. It reads the store only through the
  reader and writes nothing to it.
- With a `redaction` policy, the ask service and `cortex eval run --runner mcp` show the model only
  redacted store contents: one placeholder mapping per answer, held in memory; a result in which a
  `refuse_if_left` class is still detected is withheld from the model.
- Every answer is kept as an answer record with its tool calls, latency, tokens and outcome.
  Readers vote on it (`POST /ask/{id}/vote`, one changeable vote per identity); an answer voted
  down, failed or slower than `slow_after_ms` opens a review item (`GET /review`,
  `POST /review/{id}/close`); records older than `retention_days` are purged. On a `sqlite` store
  they live in `ask.sqlite` in the instance directory, on a `postgres` store in a PostgreSQL schema
  of their own (`ask.records`), never in EKR's. The PostgreSQL records are reached by
  `cortex ask records`, which Connectors launches with the records role's password; the service
  never reads it, and the connection requires TLS unless the URL says `sslmode=disable`.
- `cortex eval run <name> --set <file> --runner mcp|ask|replay` asks each case of an eval set and
  scores each answer with no model call, by the facts it must contain and the nodes it must cite.
- `cortex eval case add <name> --set <file> --from-review <review id>` appends the case of a closed
  review item to an eval set: the question of its answer record and the facts and nodes the
  reviewer expected, with `review_id` naming the item. An open item, an unknown one and one the set
  already holds, and one closed with no expected fact or node, are refused, and the set file is left
  as it was. `POST /review/{id}/close` takes the expectations as `expect_facts` and `expect_nodes`
  beside `reason`.

## 0.5.6 — 2026-10-17

### Added

- `cortex schema <name> --review` has the model propose ontology changes and records each one
  `pending` in `schema/<stamp>/proposals.jsonl`, applying none. `cortex schema approve <name>
  <stamp> [--only <n>…]` and `cortex schema reject <name> <stamp> [--only <n>…] --reason <text>`
  decide each proposal once. `cortex schema apply <name> <stamp>` applies exactly the approved
  proposals, each planned again against the ontology at the head (a type added meanwhile is
  recorded `invalid`), without a model call: on a `postgres` store inside an EKR stage published
  only when the spec's `gate` passes and abandoned otherwise, on a `sqlite` store after a snapshot
  `cortex restore` puts back. An apply that fails leaves its proposals `approved` and the store as
  it was. `cortex schema <name>` without `--review` behaves as before.
- A schema proposal can widen an existing edge type: `widen_edge_type` names the edge type and the
  node types to add to its sources and targets. cortex writes EKR's `WidenEdgeType` with each end
  whole, and records a widening of an unknown edge or node type, or one that adds nothing,
  `invalid`. After it is applied, a fact joining the newly admitted types is no longer refused.

## 0.5.5 — 2026-10-17

### Fixed

- A rare name in underscore italics (`_Rowan_`), in a file name after an underscore
  (`CV_Rowan.pdf`, `2024_Quill.pdf`, the `Quill` of `Rowan_Quill`) or written straight after CJK
  text (`会议上Rowan发言了`) is replaced by `RareName` and found by `refuse_if_left: [RareName]`
  (issue #74). Before, the underscore or the CJK letter before it made it part of a longer word,
  so it reached the model and the check did not see it. An underscore and a letter of a script
  without case now bound a word, as a space does. A word glued directly to a lower-case letter
  (`useFoo`) is still part of an identifier, and so is a word after underscores when the run of
  letters and digits before them begins with a lower-case letter (`snake_case_Identifier`). A
  word now also ends at a CJK letter after it, so `Rowan` is counted alone; a Latin letter
  without case (`ǀ`, `ʔ`) stays part of a word (`Nǀaqriaq`). The rare head
  of a word whose tail is a known name (`Schmidt` of `Schmidt-Müller`) has been replaced since
  0.5.4; a test now holds it.

## 0.5.4 — 2026-10-16

### Fixed

- An extraction batch fits the model it is sent to (issue #72). Behind a gateway with a 64k-token
  window, 60,000 characters of merge-request text were too long for the model (`Prompt is too
  long`) or drew an answer cut off at the output limit, and each such run counted as a failure
  of the source. Now `model.batch_chars` (at least 1,000) sets the batch size, and without it a
  gateway's `max_context_tokens` sets it, less the output reserve (the new
  `model.gateway.max_output_tokens`, passed to `claude` as `CLAUDE_CODE_MAX_OUTPUT_TOKENS`, else
  a quarter of the window) and the request's fixed part. A batch `claude` answers as too long, or
  whose answer was cut off (`Prompt is too long`, or an OpenAI-compatible server's `maximum
  context length`), is split in two and asked again, at most 3 splits deep. One document that
  alone still does not fit is left out, named in the run's new `too_large` field and `cortex.log`,
  and marked seen until its text changes; the run goes on. A part of several documents that still
  does not fit ends the run as the new outcome `batch-too-large`, exit status 6, which
  `--record-failure` does not count: the batches before it stay applied and a later run extracts
  the rest. The size counts UTF-8 bytes of document text, as the 60,000 before it did. A run's
  report and `cortex.log` line state `batch_chars` and `splits`, as does the `SourceRan` event.
- A capitalised word with hyphens that a batch holds more than `rare_limit` times is kept whole
  by `RareName` (issue #71). Before, the part after its first hyphen was replaced as a rare name
  (`Co-[Name-1]` for a frequent `Co-Authored-By` trailer), and with `refuse_if_left: [RareName]`
  the part left standing (`Co`) was then found as a rare name, so the whole batch was refused.
  Now no part of a word is a rare name unless the whole word is, and the `refuse_if_left` check
  reads words the same way. A hyphenated name glued to an underscore or to CJK text
  (`_Jean-Luc_`) still has its rare tail replaced; only a word glued to a lower-case letter, an
  identifier such as `useFoo-Bar`, is skipped whole. The rare head of a word before a span another
  class hides (`Co` of `Co-Jane`, `Jane` a known name) is replaced too, so the check does not
  refuse it, and a text cut at `max_chars` inside a frequent word (`Co-Auth`) is not refused.
- A transient Connectors failure of `connections revalidate` no longer counts against a source.
  A container that started a run within a minute of its own start saw `connections revalidate`
  answer `unavailable` at `dispatch` (`kind: operational`, `next_action: retry_explicitly`); the
  run failed as `fetch-failed`, was counted, and two such runs disabled the source
  (beyond10x/connectors#174). Now a failure Connectors marks retryable, from the revalidation or
  the `connections list` read before it, is tried again: at most 4 attempts, pausing 5 s, 10 s,
  then 20 s (`CORTEX_REVALIDATE_ATTEMPTS`, `CORTEX_REVALIDATE_PAUSE_MS`). One that remains before
  the run fetched anything fails the run as the new outcome `connection-unavailable`, exit
  status 5, which `--record-failure` does not count and which never disables the source; its
  status, metric and hook report it. A refusal is not retried and is counted as before.

## 0.5.3 — 2026-10-16

### Fixed

- Behind a model gateway (`model.gateway`), runs are bounded in tokens only, and no run stops
  over a dollar spend that never happened. `claude` prices a gateway's answer at its default
  model's rate, so `budget_usd` and `daily_ceiling_usd` could stop runs of a self-hosted or
  otherwise unpriced model. Now, with a gateway, the cost `claude` reports counts against no
  `budget_usd`, no `daily_ceiling_usd` and no dollar budget of `cortex quality` or
  `cortex schema`, `--max-budget-usd` is not passed to `claude`, and a run gate check on
  `cost_usd` checks nothing, so it no longer undoes a run over that cost. The cost is still
  recorded as reported, marked as not counted: a run's report and `cortex.log` line carry
  `cost_counted: false`, the `SourceRan` event `cost_counted`, and `spend.json` sums it in
  `uncounted_usd`.

### Added

- With `model.gateway`, `model.budget_usd` may be omitted, and `model.budget_tokens` is required:
  `create`, `update` and `adopt` refuse a gateway spec without it (exit status 2, naming
  `model.budget_tokens`, before anything is written). Without a gateway `model.budget_usd` stays
  required, and a spec without it is refused naming `model.budget_usd`. A gateway spec that still
  gives `budget_usd`, `daily_ceiling_usd` or a gate check on `cost_usd` is accepted, and
  `create`, `update` and `adopt` print one line on stderr saying they bound nothing behind the
  gateway. `create`, `update` and `adopt` also refuse a `budget_usd` or `daily_ceiling_usd` with
  more digits than a finite number holds (exit status 2, naming the field); before, such a value
  was read as an infinite budget. An instance created with a gateway by an earlier release, whose
  spec has no `budget_tokens`, keeps running with its dollar limits counted as before, until an
  `update` gives it one; an instance's own spec is not held to the finite rule either.

## 0.5.2 — 2026-10-16

### Fixed

- A `structured` source writes each mapped property in the value kind the store holds for it.
  Before, cortex declared and wrote every mapped property as a `String`, so a batch mapping a
  property the store declared `Integer` or `Boolean` was refused whole on a PostgreSQL store, and
  on a SQLite store the property was redeclared `String`. A property mapping takes an optional
  `value_kind` (`String`, `Integer`, `Decimal`, `Boolean`, `Timestamp`); without one, cortex uses
  the kind the store's ontology declares for that property of the node type, and `String` when it
  declares none. Each value is converted to the kind (`"4"` to `4`, `"false"` to `false`, RFC 3339
  text to a `Timestamp`). A record whose value does not convert is left out of its batch alone and
  named, with its key and the property, in the run's new `unconverted` field and in `cortex.log`;
  the rest of the batch is applied. The record is marked seen, so later runs skip it (and it no
  longer counts against `max_documents_per_run`) until its source text changes. A `value_kind` that differs from the kind the store declares is
  refused when the spec is read (`create`, `update`, a run), naming the node type, the property
  and both kinds.

- A run whose `ekr apply-extraction`, or the commit of the transaction that ends dropped values,
  answers that its outcome is unknown ("publication outcome is unknown") no longer counts against
  the source: it fails as `store-outcome-unknown` with exit status 4, as an unknown stage verb has
  since 0.5.1. In a stage (a gated run on `postgres`) the run leaves its stage in `stage.json`, and
  the next run abandons it and puts `state/` back; a gated `sqlite` run that had kept its snapshot
  is undone: the snapshot is restored and `state/` put back, as for a run its gate refuses, and
  the run's `cortex.log` line says so, so no batch stays in the store without being held to the
  gate; outside these (an ungated run) the batch is not marked seen, so the next run extracts it again: nothing is
  lost, and a batch that did land is written a second time, a second set of assertions and
  evidence, as any re-extraction writes it. Before, such a
  run was `apply-refused` and counted, and a second one disabled the source.

## 0.5.1 — 2026-10-16

### Fixed

- A run on a `postgres` store whose `ekr stage begin` or `ekr stage publish` answers that its
  outcome is unknown ("publication outcome is unknown") no longer counts against the source. It
  fails as `store-outcome-unknown` with exit status 4; `--record-failure` records nothing, so a
  store fault never disables a source, and the status file, metric and failure hook name the
  outcome. A publish with an unknown outcome keeps `stage.json` and `state/` as they are, and the
  next run publishes that stage at the same head; a stage whose begin was unknown is abandoned by
  the next run. Before, such a run was `apply-refused` and counted, and a second one disabled the
  source.

## 0.5.0 — 2026-10-16

### Added

- `model.gateway` sends the model calls through a gateway: `base_url` (an http:// or https://
  URL) is passed to `claude` as `ANTHROPIC_BASE_URL`, the value of the environment variable that
  `token_env` names as `ANTHROPIC_AUTH_TOKEN`, read when each call is made, and the optional
  `max_context_tokens` as `CLAUDE_CODE_MAX_CONTEXT_TOKENS`. A source's timer unit names the
  variable with `PassEnvironment=` and never holds the token. `ANTHROPIC_API_KEY` is always
  removed, and without a gateway the three gateway variables are removed as well. A `token_env`
  that is not a variable name, or names a variable cortex writes into a unit, any `XDG_*`, any
  `ANTHROPIC_*` or any `CORTEX_*` variable, is refused, as is a `max_context_tokens` of 0 or less.
- Limits in tokens, for models whose cost `claude` cannot state: `model.budget_tokens` bounds a
  run, a quality measurement and a schema proposal, and `model.daily_ceiling_tokens` bounds an
  instance per UTC day across its sources and commands (`spend.json` gains `spent_tokens`). A
  call's tokens are its answer's input, cache and output tokens together. An answer whose tokens
  cannot be counted (no `usage`, a count below 0, a sum that does not fit) stops the run before its
  next batch under either limit, without reporting the daily ceiling reached.

## 0.4.0 — 2026-10-15

### Added

- A spec may name a Connectors connection by `profile` of its adapter instead of by id
  (`profile: tavily.api-key`), on a source and on a `postgres` store's `connection` and
  `schema_connection`. Each command resolves it to the one connection of that adapter and profile
  that is not `revoked`; none or several is refused naming the adapter, the profile, the count and
  every matching id. A `postgres` store's viewer unit holds the id resolved when it was written,
  and `cortex update` rewrites it after a reconnect. The id form stays valid.
- The `NextLink` paging style reads the next page's cursor out of a link in the page body
  (`next: _links.next`), and `stop_older_than_since: true` ends a walk at the first record whose
  `time` is older than the run's `{since}`; the rule needs a `time` and is refused without one.
- `cortex run <instance>/<source> --fetch-only` fetches a source as a run would and prints its
  window and how many fetched documents are new, changed, held or unchanged, with no model call and
  no write to the store, the seen-state or `cortex.log`. A disabled source is fetched too.
- A run's result and `cortex.log` line carry `model_calls`, the input, cache and output tokens the
  calls reported, and an estimate of how the input splits between each prompt's fixed part and the
  documents (`fixed_input_tokens`, `documents_input_tokens`).
- `cortex adopt --seen <source>=<file>` takes entries with a key and `applied_at` and no `hash`:
  the source's first run records the hash of each such document it fetches and does not extract
  it. A key is the one cortex gives the document (for a files source, the file's absolute path).
  An entry with no `applied_at`, or whose fields are not of their types, is refused naming its key.

### Changed

- The specification requires ess 0.60.0.

## 0.3.1 — 2026-10-15

### Added

- `{since_epoch}` and `{until_epoch}` in a string value of a Connectors or structured source's
  `inputs` are replaced by the run's window as whole seconds since the Unix epoch, the same
  instants `{since}` and `{until}` name, for an operation whose bounds are epoch seconds. Any other
  text in braces is passed to the operation as written.

## 0.3.0 — 2026-10-15

### Added

- A run on a PostgreSQL store held to a gate can be undone: it runs in an EKR stage (`ekr stage
  begin`), which is published when the gate passes and abandoned when it fails, with `state/` put
  back. A stage a killed run left is settled by the next run. SQLite runs and runs without a gate
  are unchanged.
- `model.daily_ceiling_usd` holds an instance's model calls, across all its sources and
  `quality` and `schema`, to a spend per UTC day (`spend.json`). A run that reaches it stops
  before its next batch, keeps what it applied, and counts no failure.
- `monitoring` in the spec file: `metrics` writes a Prometheus text file after every run, and
  `hook` calls a command once per failed run. `cortex sources` shows each source's last outcome,
  its time and its reason.
- `cortex run` exits by outcome: 0 for `ran`, 1 for a failed run or an unknown source, 3 for a
  disabled source, 2 for an error before any outcome.

### Changed

- When `claude` answers an error, the run's reason and `batch-<n>/claude/error.txt` keep the text
  it gave, masked and bounded. An error answer that cost nothing is asked once more after a
  minute before the run fails.
- `cortex setup` installs EKR 0.0.34 by default, and the examples pin it.

## 0.2.3 — 2026-10-13

### Changed

- `cortex setup` installs EKR 0.0.33 by default, and the examples pin it.
- cortex is specified and generated with ESS 0.59.0; CI checks with ESS 0.59.0 and aep 0.71.2.

### Fixed

- A `claude` that exits before reading the prompt is reported by its exit status and its own
  error output, not as `cannot send the prompt: Broken pipe`.

### Documentation

- The install commands in the README and the quickstart name the current release; they named
  0.2.0 after 0.2.1 and 0.2.2 were out.
- `STATUS.md` states the repository's state against named targets, each row dated and naming the
  command it was read from.

## 0.2.2 — 2026-10-08

### Fixed

- A structured source whose adapter, operation, `files` path or glob ends in a credential name
  (`vault.secret`) no longer applies its unchanged records again on every run: that part of its
  identities is written in hex, and values applied under the earlier identities still end. Sources
  whose prefix masking leaves alone keep their identities.
- The documentation site waits until GitHub lists the finished build run before it publishes.

### Documentation

- Install from the release tarball checked against `SHA256SUMS`; a Use cases page; the Codex
  backend is documented as unavailable; EKR limits re-checked against 0.0.32.

## 0.2.1 — 2026-10-08

### Fixed

- A run retries a Connectors invocation once when Connectors answers `timeout` at `admission`
  (its own 30 s deadline ran out); a second timeout answers `fetch-failed` as before, and nothing
  else is retried.

## 0.2.0 — 2026-10-07

### Added

- A `structured` source with a Connectors input can call `children` operations per parent record
  and map each child record to its own node, linked to its parent by the edge its `parent:` names and
  dated by its `time` field. A child call that fails for one parent is named in `skipped`, and that
  parent's earlier children of the operation are kept.
- A `structured` source can link each merged change to the first tag that ships it: a `links`
  entry names the tag and change child operations and a compare operation, cortex calls the compare
  once per tag (from the tag before it), and each change gets one edge, named by the link's
  `relation`, to the earliest tag whose comparison holds it. A change merged before its tag gets its
  edge in the run that first sees the tag. A failed compare call or a tag with no usable `order`
  value is named in `skipped`, and that parent's edges already stored are kept.

### Changed

- cortex is specified and generated with ESS 0.55.0 (was 0.52.0); `spec/suite.json` is
  `ess-conformance/34`, and the conformance runner checks `expect_event_values` steps.
- The extraction prompt names each existing property with its value kind
  (`Release(version: String, release_date: String)`), so the model answers in the kind the store
  declares and EKR rejects fewer facts as `extraction-value-mismatch`.
- The `QualityMeasured` event carries the measurement's `rate` and `cost_usd`, and
  `SchemaChangesProposed` its `cost_usd`; `cortex quality` and `cortex schema` print them from the
  event, under the same keys and in the same forms as before.

## 0.1.0 — 2026-10-07

### Added

- `cortex schema <instance> [--sample N] [--dry-run]` proposes ontology changes from a sample of facts
  and evidence and applies the ones EKR accepts; every proposal is recorded under `schema/<stamp>/`.
- `cortex quality <instance> --sample N` judges a sample of facts against their evidence with the
  instance's model and reports a pass rate with its interval under `quality/<stamp>/`.
- A `structured` source reads JSON files, and `dropped: Supersede` ends what a source no longer
  lists: a changed value is superseded, a value no longer listed is retracted.
- The model marks a property value a document says changed as `replaces: true`, so the older value is
  superseded; a run names an EKR validator rejection by its issue codes (`invalid-supersession`).
- Facts are dated by their document's time: evidence `observed_at` comes from the document's `time` or
  `published` (RFC 3339, a bare date or epoch seconds), else the run start.
- A `records` spec can map its author field and inline `<@id>` mentions through a JSON lookup file
  (`lookup`), and can name `fallback_text` templates used when `text` renders empty.
- `<home>/bin/cortex`, the copy every timer of a home runs, has its version recorded beside it.
  `create`, `update` and `adopt` replace it only with the same or a newer cortex and say so in
  their `binary` detail; an older cortex keeps it, naming both versions, unless `--replace-binary`
  is given. `cortex list` shows the version as `binary_version`.
- The test harness no longer fails with `Text file busy`: stand-ins are written by a child
  shell, never held open for writing by a test process.
- A run gate: `gate.checks` bound the run report's numbers or `ekr quality` measures, and a run
  that fails its gate is undone from the snapshot taken before it. A restore now writes into the
  live store, so an attached `ekr mcp` reader no longer blocks it.
- `cortex adopt` turns an existing EKR store into an instance without reseeding: a SQLite store is
  copied with its `-wal`, a PostgreSQL store is used in place, `--host` passes the store's own host
  document and `--seen` carries what each source has seen.
- A files source can yield one document per JSON line or per markdown section, with ids, authors
  and threads; unreadable lines and files are named in `skipped`.
- Run snapshots: a run that applies something first copies the store and `state/`; `cortex restore`
  puts both back after snapshotting the current state, so a restore can itself be undone.
- `examples/agent-tooling.yaml`: a web instance that searches agent-tooling news daily and crawls
  the MCP specification weekly.
- Scheduled runs work with nobody at the keyboard: timers carry the `connectors`, `claude` and
  `codex` paths and `HOME` cortex was created with, and a relative home or a spec given as a bare
  file name no longer breaks them. Run `loginctl enable-linger` once to keep timers running while
  logged out.
- Redaction classes `Url`, `known_names` and `RareName` replace links and personal names with
  placeholders the model sees and the store does not keep; `Credential` masks secrets for good.
  `refuse_if_left` fails a run, before any model call, when a listed class survives.
- A `structured` source imports records from a Connectors operation with no model call and no
  cost: paths map record fields to a node, its aliases, properties and relations. A record's
  identity is `<adapter>:<operation>:<id>`; nodes are named `<name> (<identity>)`, and a plain name
  merges only when the spec maps it as an alias.
- Credentials are masked in a document's title, description and URL as in its text, including
  percent-encoded names, URL passwords and prefixed names such as `client_secret=`; documents
  whose URLs differ only in a credential are one document.
- Connectors sources walk every page (`Token`, `PageNumber`, `Keyset`), call a child operation
  per parent and append its records, and read a window: `{since}`/`{until}` in the inputs open
  at the last successful run (with a 5-minute overlap) and stay open for changes held back,
  runs that stopped and child calls that failed, so no record is skipped. A child call that
  fails on 3 runs in a row is skipped.
- A seed directory that is a symlink, and symlinked files inside one, are copied as regular files.
- An instance's store lives on the backend its spec names: SQLite as before, or PostgreSQL through
  an `ekr.postgres/1` file (`cortex create --postgres-schema-config` provisions the schema). A
  create refuses a tenant that already holds a store; refusals name fields, never their values.
- Redaction before the model: emails, phone numbers, card numbers, IP addresses and spec rules
  become placeholders such as `[Email-1]` in everything the model sees, and are restored before
  the extraction is applied, so the store keeps the real values. A rule with a `replacement` is
  irreversible. Runs log `redacted` and `unrestored` counts, and `cortex run` and `cortex create`
  print them.
- The specification declares what a source has seen (`SeenDocument`) and the seed digest an
  update compares; `cortex list` prints `seed_digest`. An update naming a `..` or absolute path,
  or a renamed seed file, is a seed change.
- The `cortex` command line: `create`, `update`, `remove`, `run`, `source add`, `source enable`,
  `list`, `sources`, `mcp-line`, `setup` and `schema`. Each command prints its outcome as one JSON line;
  a refusal exits 1, a usage or environment failure exits 2.
- Instances: one EKR SQLite store per instance under `$CORTEX_HOME` (default
  `~/.local/share/cortex`), seeded from the spec file's schema, with a viewer and one systemd user
  timer per source. Two failed runs of a source in a row disable its timer; `cortex source enable`
  turns it back on.
- Sources: `web` (search queries or listed sites, read through any Connectors adapter implementing
  `datasource.websearch/v1alpha1`, `tavily` by default), `connectors` (the records of any
  operation a Connectors adapter admits) and `files` (local files matching a glob). A source names
  a Connectors connection id; cortex holds no secret.
- Runs keep only new or changed documents, mask credential shapes, mint the evidence for every
  document, extract facts and relations through an isolated `claude -p` call checked against
  `ekr schema`, refuse a fact citing evidence cortex did not issue, and apply the result with
  `ekr apply-extraction`.
- Connectors answers are read as the CLI returns them, and a connection whose validation lapsed
  is revalidated before a read and renewed once when a read is refused at admission.
- The specification of cortex in ESS (`spec/`, ESS 0.52.0), with the model, the spec file's serde
  types, its JSON Schema and 35 conformance scenarios generated from it.
- A documentation site, published at <https://beyond10x.github.io/cortex/>.
- CI: `task check` and the planning-store validation on every pull request and push to `main`.
- A release workflow: a published GitHub Release `v<version>` gets
  `cortex-<version>-x86_64-unknown-linux-gnu.tar.gz` and `SHA256SUMS` attached; a manual run
  builds the same two files as a workflow artifact.
- A PostgreSQL store can name a Connectors `connection` (and a `schema_connection` for
  provisioning): every `ekr` that opens the store then starts through
  `connectors connections launch --consumer ekr` and reads the password from descriptor 3 as its
  `password_file`, so no file cortex writes holds it.

### Changed

- The EKR pin is 0.0.32 (`cortex setup` default, the examples and CI): its `ekr.postgres/1` takes a
  `password_file`.

### Fixed

- A second home no longer takes over or deletes another home's systemd units for an instance of
  the same name. Each unit records its home as `X-CortexHome=`; `create`, `update` and `adopt`
  refuse a unit another home wrote, naming that home, and `remove`, `restore`, `source enable` and
  `--record-failure` leave it alone. Units written by an earlier cortex are read as belonging to
  the home their service runs. A unit whose home was deleted, or holds no active instance of that
  name, is taken over by the next `create`, `update` or `adopt` and named in `units.taken_over`.
  A home path holding a line break is refused by every command, and no unit is written with a
  value holding one.
- A source named `entities` or `seed` is refused by `create`, `update` and `adopt`, naming the
  reserved name. Its seen documents would have shared `state/entities.json` with the known entity
  names, or `state/seed.json` with the seed, and a source named `entities` was disabled after
  another source ran.
- Evidence over 16 KB is no longer lost: each evidence payload is cut, at a character boundary, to
  the 16,384 bytes EKR 0.0.30 takes, on a source run and on the seed alike, and the text the model
  is shown is cut to what the payload holds. A structured record's evidence leads with its mapped
  values, so the cut never takes a value its facts cite; a record whose mapped values alone are
  over the bound is named in `skipped` and not applied. A run names each document a part EKR
  rejected belongs to, with EKR's refusal, in `rejected`.
- A seed extraction that stops before every seed document is extracted (its budget spent, a model
  call or an apply failed) makes `create` answer `partial` instead of `created`, with the seed's
  `documents_new`, `documents_applied`, `stopped`, `facts_refused` and `parts_rejected`, or
  `failed`. The documents it left are not recorded as seen, and `cortex run <name>/seed` extracts
  them, keeping a snapshot first and held to the gate as a source run is; it refuses an adopted
  instance. A seed that finishes still answers `created`.
