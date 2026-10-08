---
format: aep.planning-md/3
id: review-result:adversary-structured-compare-links-pass-1
kind: review-result
status: archived
title: 'Adversary pass 1: structured-compare-links (wave 20261007d)'
relations:
- reviews: story:structured-compare-links
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:37Z", actor: "human:timo", revision: 2}
---
```
unit: U1 story:structured-compare-links: commit 035e597 (unit/structured-compare-links) plus untracked tests/structured_compare_adv.rs
verdict: NEEDS-CHANGE
cases: executed 8→18, red 2
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: b10x-gates scan-text on tests/structured_compare_adv.rs (it exits 1 here, "protected file unavailable", without the gates policy and key)
```

**1. Diff proof.** `git --no-pager diff --stat` is empty. `git status --short` shows only `?? tests/structured_compare_adv.rs`, a test file. No source, spec or generated file was touched.

**2. Cases added**, in `<worktree>/tests/structured_compare_adv.rs`:

| case | asserts | now |
|---|---|---|
| `adv_a_change_merged_before_its_tag_gains_its_edge_when_the_tag_appears_under_a_refresh_window` | with `refresh_after_days: 1`, c1 has its edge to v1 after the run that first sees v1 | **red** |
| `adv_a_failing_link_does_not_keep_a_change_its_parent_no_longer_lists` | Supersede, tag v2 has no `created_at`, c2 removed from the list: after 2 runs c2's edge to the project is gone and c1's edge to v1 is kept | **red** |
| `adv_tags_are_ordered_by_their_instant_not_their_name_or_local_time` | tags v8 (with a +05:00 offset), v9, v10 are compared in time order; c8 goes to the earliest; an unchanged Supersede run ends nothing | green |
| `adv_a_paged_compare_links_the_changes_of_every_page` | a change on page 2 of the compare is linked | green |
| `adv_a_compare_with_pages_left_is_skipped_and_nothing_after_it_is_linked` | `max_pages` reached → named in `skipped`, no later range asked, earlier link kept | green |
| `adv_tags_left_unread_make_no_compare_call_and_are_skipped` | tags with a page left unread → no compare call, "not all read" in `skipped` | green |
| `adv_a_parent_with_tags_and_no_change_makes_no_compare_call` | the doc's "no change read, no compare call" | green |
| `adv_ids_masking_would_change_link_through_clean_identities_and_are_never_stored` | parent `top-secret`, a credential-shaped tag name and change id: links land, every identity is mask-clean, no raw value under the cortex home | green |
| `adv_a_link_a_change_no_longer_has_is_ended_under_supersede` | c1 moves from v1 to v2: 1 retracted, one edge, to v2 | green |
| `adv_a_change_applied_before_its_tag_names_the_node_the_tag_later_becomes` | changes child listed first, `max_documents_per_run: 2`: one v1 node, one edge | green |

Red output, each case run alone (`alone.sh`, `--exact`) before any suite run:
```
thread 'adv_a_change_merged_before_its_tag_gains_its_edge_when_the_tag_appears_under_a_refresh_window' (3138144) panicked at tests/structured_compare_adv.rs:464:5:
assertion `left == right` failed: the run that first sees v1 gives c1 its edge: {"command":"run","detail":{"cost_usd":"0.0000","documents_applied":1,"documents_new":1,"facts_refused":0,"masked":0,"parts_rejected":0,"source_id":"r/projects","stopped":null},"outcome":"ran"}
  left: []
 right: ["forge:projects.list/tags.list:1:v1"]
thread 'adv_a_failing_link_does_not_keep_a_change_its_parent_no_longer_lists' (3140282) panicked at tests/structured_compare_adv.rs:512:5:
c2 is no longer listed and its edge to the project is no link, yet two runs keep it: [("01a11874-416a-7070-b73d-de20ec9a2f82", "IN_PROJECT", "01a11874-4169-7325-85d8-0f60b21f920f"), ("01a11874-416a-7070-b73d-de221434dbcb", "IN_PROJECT", "01a11874-4169-7325-85d8-0f60b21f920f"), ("01a11874-416a-7070-b73d-de221434dbcb", "shipped_in", "01a11874-416a-7070-b73d-de20ec9a2f82"), ("01a11874-416a-7070-b73d-de242a675b3b", "IN_PROJECT", "01a11874-4169-7325-85d8-0f60b21f920f"), ("01a11874-416a-7070-b73d-de242a675b3b", "shipped_in", "01a11874-416a-7070-b73d-de20ec9a2f82"), ("01a11874-4332-71e2-8833-86dbd4d66f46", "IN_PROJECT", "01a11874-4169-7325-85d8-0f60b21f920f")]
```
The masking case's first solo run also failed, but on my own helper: it assumed `<name> (` is a node's first alias, and EKR sorts aliases. I fixed the helper and re-ran that case alone; it passed (exit 0). It is not a finding. After `rustfmt`, the two red assertions moved to :463 and :511.

**3. Commands and exit statuses** (all with `TMPDIR=<scratch>/tmp`, `nice -n 19`, `CARGO_BUILD_JOBS=8`, `CORTEX_TEST_EKR=<worktree>/target/ekr-0.0.32/bin/ekr`):
```
worktree hook session-start --path <worktree> --session cortex-w20-a-adv-1        EXIT=0
alone.sh: 9 cases, each `cargo test --locked -p cortex-cli --test structured_compare_adv -- --exact <case>`
  refresh_window=101  failing_link=101  ids_masking=101 (my helper)  the other 6=0
alone.sh ids_masking (helper fixed)                                                EXIT=0
alone.sh adv_a_link_a_change_no_longer_has_is_ended_under_supersede                EXIT=0
cargo test --locked -p cortex-cli --test structured_compare --test structured_compare_adv   EXIT=101
cargo clippy --locked -p cortex-cli --test structured_compare_adv -- -D warnings   EXIT=101 (cloned_ref_to_slice_refs in my file; fixed)
rustfmt --edition 2021 --check tests/structured_compare_adv.rs                     EXIT=1 (formatted)
cargo clippy --locked -p cortex-cli --test structured_compare_adv -- -D warnings   EXIT=0
cargo fmt -p cortex-cli -- --check                                                 EXIT=0
cargo test --locked -p cortex-cli --test structured_compare --test structured_compare_adv   EXIT=101
  structured_compare:     test result: ok. 8 passed; 0 failed
  structured_compare_adv: ...refresh_window ... FAILED   (panicked at :463:5)
                          ...failing_link ... FAILED     (panicked at :511:5)
                          test result: FAILED. 8 passed; 2 failed
b10x-gates scan-text tests/structured_compare_adv.rs                               EXIT=1 "protected file unavailable"
worktree hook session-end --path <worktree> --session cortex-w20-a-adv-1           EXIT=0
```
The red suite is the result this pass was looking for. Every test name it printed exists in this tree.

**4. Findings** (covering 035e597 plus my untracked test file):

| # | file:line | sev | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|---|
| 1 | website/docs/spec-file.md:454 | warning | NEEDS-CHANGE | introduced | :463 is red. After run 2, c1 has no edge (`documents_new 1`, only the tag applied). The link sits in the hashed text (src/run.rs:381), and `SeenState::wants` (src/state.rs:140) holds back any changed record for `refresh_after_days`. The edge comes late by up to that many days; it is not lost. | Any structured source with `refresh_after_days` of 1 or more. spec-file.md:148-149 recommends that; the examples use 7 and 30; every implementor test uses 0. |
| 2 | src/run.rs:355 | warning | CONFIRMED | introduced | :511 is red. A link failure holds the whole `<changes>:<parent>:` prefix, so with Supersede no value of any of that parent's changes is ended (here, a removed change). The wave decision only holds stored edges. spec-file.md:466 documents the wider hold, so docs and code agree; the gap is against the decision. Unlike children (`CHILD_FAILURE_LIMIT`), there is no limit, so a permanent failure holds forever. | A failure that never clears: a tag record missing its `order` field, a compare the provider refuses for one project, or a provider that refuses the empty `from` the first range sends. |
| 3 | tests/structured_compare.rs | note | CONFIRMED | introduced | Four changes to the code the suite would not catch. This is inferred from the fixtures; I did not build the mutated versions. (a) sorting by name only at sources.rs:693: every fixture pair's names sort the same as its times (:284-285, :399-400, :422-423, :480-481; :375 has equal times). (b) dropping the compare paging at :711: the file has 0 `paging`. (c) dropping the `whole` check at :572 and (d) dropping the empty-changes return at :666: no test reaches either. My four green cases now catch all four. | The suite itself. |

Fix for 1: either say the delay in the Links paragraph and the wave decision, or don't apply the refresh hold to a change whose only difference is its `cortex.links` member.

Fix for 2: hold only the link-relation assertions of that parent's changes (prefix plus relation), and let every other value end.

**5. Attacked and could not break:**
- Ordering by instant: offsets, ties, and earliest-tag-wins.
- Compare paging, and the pages-left failure stopping later ranges.
- Tags not all read: no call, named in `skipped`.
- No change read: no compare call.
- Credential-shaped parent, tag and change ids: hex and digest identities link correctly, nothing raw is stored.
- Supersede ends a link the change no longer has; an unchanged Supersede run ends nothing.
- A change applied before its tag's node exists.
- Spec refusal and redaction: only read (src/spec.rs:185-197; new `skipped` text goes through `masked_key`; a structured source never calls the model). Not tested.

**6. Paths written outside the worktree:** none. Scratch is all inside the worktree, under `<worktree>/target/wave-scratch/adv/`: `alone.sh`, `alone{,2,3}.log`, `suite{,2}.log`, `clippy{,2}.log`, `fmt{,2}.log`, `tmp/` (56K in all).

```findings
- file: website/docs/spec-file.md
  line: 454
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: with refresh_after_days of 1 or more, a change applied before its tag is held back by the refresh window, so it does not gain its edge in the run that first sees the tag as the docs and the wave decision say (tests/structured_compare_adv.rs:463)
- file: src/run.rs
  line: 355
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a link failure holds every value of every change of the parent, not just stored link edges, so under Supersede a change the parent no longer lists is never ended while the failure lasts, with no failure limit (tests/structured_compare_adv.rs:511)
- file: tests/structured_compare.rs
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the suite does not catch sorting tags by name only (src/sources.rs:693), dropping compare paging (:711), dropping the tags-whole check (:572) or dropping the empty-changes return (:666); inferred from the fixtures, not from mutated builds
```

(Recorded by the coordinator. One change from the report as returned: the worktree's absolute path in § 2 is written `<worktree>`, as the report itself does elsewhere, because the repository refuses home-directory paths. The case file was committed on the unit branch as da7c46f.)
