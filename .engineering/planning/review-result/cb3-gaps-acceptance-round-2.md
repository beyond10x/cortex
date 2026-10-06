---
format: aep.planning-md/3
id: review-result:cb3-gaps-acceptance-round-2
kind: review-result
status: active
title: 'Plan critic (acceptance), round 2: cb3 gap stories'
relations:
- reviews: story:structured-children
- reviews: story:extraction-supersedes
- reviews: story:postgres-run-undo
- reviews: story:record-field-lookups
revision: 1
---
needs-revision
story:served-channels-by-audience — acceptance 2 searches with `ekr search` for a phrase that occurs only in the restricted channel's message text, but `ekr search` matches node names and aliases only and "does not search evidence text", so it answers no node whether or not the text was written and the check passes on a store that leaks — .engineering/planning/story/served-channels-by-audience.md:29 (EKR `docs/cli.md:1074-1075`)
story:served-channels-by-audience — acceptance 1 says "a check refuses a channel with no class" but names no check, command or test, and joins it to the listing outcome with "and" — .engineering/planning/story/served-channels-by-audience.md:28
story:cortex-host-reaches-staff-db — acceptance 2 offers "a no-op seeded-store probe that writes and rolls back (or `ekr validate` with the application role)", but `ekr validate` takes a transaction id and validates it, so it does not show the role can write. The probe is not named, so the two branches prove different things and the check can pass without proving write access — .engineering/planning/story/cortex-host-reaches-staff-db.md:29
story:postgres-store-backup — acceptance 2 compares the restore with "the source at backup time", but nothing in the story records the head and snapshot root at dump time, so on a live store two readers get different answers and the check cannot be read twice — .engineering/planning/story/postgres-store-backup.md:28
story:retire-local-redaction — the folded `task secrets` outcome is now only "call the shared detector", which names no detector and drops the archived story's observable (the scan exits non-zero on a staged file with a runtime-assembled credential shape and zero on a clean tree, with `src/redact.rs` absent) — .engineering/planning/story/retire-local-redaction.md:45 (archived `story:secrets-scan-without-redact` Acceptance)
story:read-answers-name-types — the Outcome says "Every read that answers a node type, edge type or property id carries its name", but the acceptance covers only `search`, `describe_node`, `expand`, `timeline` and `explain`, so `changes_since`, `resolve` and `describe_evidence` can close unchecked, or the Outcome should name the five reads — .engineering/planning/story/read-answers-name-types.md:32 against :39-47

Round-1 findings:

| Round-1 finding | Status |
|---|---|
| structured-children: renamed-project clause had no before state or trigger, and was a fourth outcome in one sentence | resolved; the acceptance is now 3 numbered checks, and item 3 has a run 1, a run 2, and an alias after run 2 |
| record-field-lookups: first-non-empty text field untested | resolved; acceptance 2 |
| read-answers-name-types: only `search` checked, "a type read" unnamed | mostly resolved; five reads, the `type_name` field and `describe_type` are named. Residue is the Outcome-versus-acceptance finding above |
| explain-bounds-documents: no way to read the rest, cited fact not shown inside the returned bytes | resolved; `offset`/`limit`, the cited text inside 64 KiB, and a reassembly check |
| served-channels-by-audience: "answers nothing" conflicted with the 547-store Open section | resolved; the text already in the store is moved out to `decision-blocker:pilot-store-restricted-text`, and the acceptance is scoped to a fixture run |
| served-channels-by-audience: two outcomes in one sentence | partly resolved; split in two, but the "and a check refuses" clause remains (finding 2 above) |

Not findings: story:structured-children arithmetic checks out (2 + 2×(3+4) = 16). story:extraction-supersedes and story:postgres-run-undo are unchanged and still observable. `decision-blocker:pilot-store-restricted-text` names the question, who decides and what it blocks.

Read: 12 artifacts, plus 3 round-1 records and 3 supporting artifacts. I ran `aep plan artifact show` in all three stores for the 10 stories and the decision-blocker, plus `epic:run-on-cortex`, `story:retire-local-redaction` and the archived `story:secrets-scan-without-redact`. I also read `story:read-surface-specified`, `story:file-records`, `story:cortex-cutover` and the three round-1 records. In the trees I checked EKR `docs/cli.md`, `crates/ekr/src/cli/mod.rs`, the cb3 `Taskfile.yml` and cortex `src`.

Could not establish:
- Whether `changes_since`, `resolve` and `describe_evidence` actually cite type or predicate ids. The `ekr-views` structs I read were inconclusive, so that finding is conditional.
- What "named in `skipped`" means for a records source. `skipped` exists in cortex `src/run.rs:60` as a parent-skip list. I did not verify it applies to records, which is not an acceptance question.
- Whether `story:cortex-cutover` done-criterion 2 (adopt without reseed) conflicts with `decision-blocker:pilot-store-restricted-text`. The cutover story was not given to me.

Out of my lane:
- `epic:run-on-cortex` carries stale duplicate sections: "Filed for the missing rows", "Critical path to cutover" and "Pilot" each appear twice. The older copy still lists the archived `story:secrets-scan-without-redact`. That is scope and coverage.
- The archived story's body repeats its "Archived 2026-10-06" heading.
- The scope and `serves` gaps on `story:cortex-host-reaches-staff-db` are now partly filled (cited scope). They are still absent from `story:postgres-store-backup`'s `serves`, which is parallel-safety or design.

```findings
[
 {
  "file": ".engineering/planning/story/served-channels-by-audience.md",
  "line": 29,
  "category": "acceptance",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "acceptance 2 searches with `ekr search` for a phrase that occurs only in the restricted channel's message text, but `ekr search` matches node names and aliases only and does not search evidence text, so it answers no node whether or not the text was written and the check passes on a store that leaks"
 },
 {
  "file": ".engineering/planning/story/served-channels-by-audience.md",
  "line": 28,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "acceptance 1 says \"a check refuses a channel with no class\" but names no check, command or test, and joins it to the listing outcome with \"and\""
 },
 {
  "file": ".engineering/planning/story/cortex-host-reaches-staff-db.md",
  "line": 29,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "acceptance 2 offers a no-op seeded-store probe that writes and rolls back, or `ekr validate` with the application role, but `ekr validate` takes a transaction id and does not show the role can write, and the probe is not named, so the two branches prove different things and the check can pass without proving write access"
 },
 {
  "file": ".engineering/planning/story/postgres-store-backup.md",
  "line": 28,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "acceptance 2 compares the restore with \"the source at backup time\", but nothing in the story records the head and snapshot root at dump time, so on a live store two readers get different answers and the check cannot be read twice"
 },
 {
  "file": ".engineering/planning/story/retire-local-redaction.md",
  "line": 45,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the folded `task secrets` outcome is only \"call the shared detector\", which names no detector and drops the archived story's observable: the scan exits non-zero on a staged file with a runtime-assembled credential shape and zero on a clean tree, with `src/redact.rs` absent"
 },
 {
  "file": ".engineering/planning/story/read-answers-name-types.md",
  "line": 32,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the Outcome says every read that answers a type, edge type or property id carries its name, but the acceptance covers only search, describe_node, expand, timeline and explain, so changes_since, resolve and describe_evidence can close unchecked, or the Outcome should name the five reads"
 }
]
```