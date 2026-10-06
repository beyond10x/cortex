---
title: Commands
sidebar_position: 4
description: Every cortex command, its arguments and what it prints.
---

# Commands

Every command that changes something prints one JSON line,
`{"command": …, "outcome": …, "detail": …}`, and exits 0 for a successful outcome and 1 for a
refused one. An error before any outcome (an unreadable spec file, a home that cannot be locked)
goes to stderr as `cortex: <message>` with exit status 2. `cortex list` and `cortex sources` print
one JSON line per row.

The outcomes are the ones the specification declares; the
[generated reference](./reference/ess/cortex-instance.md#commands) lists each command's outcomes,
errors and events.

## Global options

| option | environment | default |
|---|---|---|
| `--home <dir>` | `CORTEX_HOME` | `~/.local/share/cortex` |
| `--connectors <bin>` | `CORTEX_CONNECTORS` | `connectors` |
| `--claude <bin>` | `CORTEX_CLAUDE` | `claude` |

## Instances

| command | does | outcomes |
|---|---|---|
| `cortex create --spec <file> [--no-extract] [--no-units] [--replace-binary] [--postgres-schema-config <file>]` | create an instance from a spec file and add its sources. `--no-extract` skips the seed documents, `--no-units` installs no systemd units, and `--replace-binary` replaces `<home>/bin/cortex` even when it is newer than this cortex (see [Operating](./operating.md#systemd-units); `adopt` and `update` take it too). For a `postgres` store, `--postgres-schema-config` names the `ekr.postgres/1` file of the schema-management role, and `ekr postgres-schema` creates the provider tables with it before the seed; without it the tables must already exist. `partial` is `created` for a seed that stopped before every seed document was extracted: its `seed` detail adds `documents_new`, `stopped`, `facts_refused` and `parts_rejected` (or carries `failed`), and `cortex run <name>/seed` extracts the rest | `created`, `partial`, `name-taken`, `connection-missing`, `seed-refused` |
| `cortex adopt --spec <file> --store <file> [--host <file>] [--seen <source>=<file>]… [--no-units] [--replace-binary]` | make an existing EKR store an instance, with its whole revision history, and add the spec's sources: nothing is seeded, extracted or written to the store named (see [Operating](./operating.md#adopting-an-existing-store)). `--store` is the SQLite database, which is copied into the instance, or for a `postgres` spec the `ekr.postgres/1` file its `store.value.config` names. `--host` is the `ekr.cli-host/1` document the store was seeded under. `--seen` starts a source from a `cortex.seen/1` file; without it the first run re-extracts every document. `adopted` reports the store's head as `revision`, and `seen_documents` and `seen_without_evidence` (seen documents the store holds no evidence of) | `adopted`, `name-taken`, `backend-mismatch`, `store-unreadable`, `store-held`, `seed-types-missing` |
| `cortex update <name> --spec <file> [--no-units] [--replace-binary]` | replace the instance's spec: sources, model and serve settings. New sources are added and every source's timer is (re)installed; the seed cannot change | `updated`, `seed-change-refused`, `not-active`, `no-such-instance` |
| `cortex remove <name>` | stop and delete the instance's timers and viewer that its home wrote (`units.kept` names any of the same name another home wrote); its directory and store stay | `removed`, `wrong-state`, `no-such-instance` |
| `cortex restore <name> <snapshot>` | put a `sqlite` store and the instance's `state/` back to a snapshot a run took before it applied anything (see [Operating](./operating.md#undoing-a-run)). It snapshots what it replaces as `<ms>-before-restore` and reports the name as `before_restore`, and `state_restored`; a running viewer is stopped and started again. It does not wait for the home's lock | `restored`, `busy`, `no-such-snapshot`, `backend-unsupported`, `no-such-instance` |
| `cortex list` | instances: name, state, description, model, EKR version, seed digest, viewer address, and the version of `<home>/bin/cortex` their timers run (`binary_version`, null when none is recorded) | |
| `cortex mcp-line <name>` | print the `claude mcp add` line that serves the instance's store through `ekr mcp` | |

`cortex update` refuses a spec whose `name` is not `<name>`. `create`, `adopt` and `update` name each unit they took over from a home that no longer holds the instance in `units.taken_over` (see [Operating](./operating.md#systemd-units)). Every command refuses a home whose path holds a line break.

## Sources

A source id is `<instance>/<source>`.

| command | does | outcomes |
|---|---|---|
| `cortex run <instance>/<source> [--record-failure]` | run one source once. With `--record-failure`, which the timers pass, a failed run is also counted. `<instance>/seed`, for an instance with no source of that name, extracts the seed documents no seed extraction applied yet, such as those a `partial` create left. Like a source run it keeps a snapshot named `<time>-seed` before it applies and is held to the spec's `gate`; on an adopted instance it answers `fetch-failed`, because the seed belongs to the store's earlier life | `ran`, `fetch-failed`, `extraction-failed`, `apply-refused`, `disabled`, `no-such-source` |
| `cortex source enable <instance>/<source>` | enable a source that two failed runs in a row disabled, and its timer | `enabled`, `wrong-state`, `no-such-source` |
| `cortex source record-failure <instance>/<source> --reason <text>` | count one failed run; the second in a row disables the source and its timer | `counted`, `disabled`, `already-disabled`, `no-such-source` |
| `cortex source add --instance-name <name> --source-id <id> --name <name> --kind <Web\|Connectors\|Files> --schedule <calendar>` | register a source in the registry; `create` and `update` do this for the spec's sources | `added` |
| `cortex sources` | sources: id, kind, state, schedule, runs, consecutive failures | |

A `ran` line's detail carries `documents_new`, `documents_applied`, `cost_usd`, `facts_refused`
(facts the model cited unissued evidence for), `parts_rejected` (parts EKR rejected), `rejected` (each document a rejected part belongs to, with EKR's refusal, or the codes of the issues validation raised, such as `invalid-supersession`; present only when there is one), `masked`
(credential shapes replaced) and `stopped` (why the run ended early, if it did). When the spec
has a `redaction` policy it also carries `redacted` (values replaced by placeholders in the
prompts sent, per class or rule) and `unrestored` (placeholders in the model's answers that had no
value to put back).

## Setup

| command | does |
|---|---|
| `cortex setup [--ekr-version <v>]` | install `ekr` at the version (default 0.0.31) with `cargo install` when it is missing |
| `cortex schema` | print the spec file's JSON Schema |
