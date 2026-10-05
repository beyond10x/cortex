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
| `cortex create --spec <file> [--no-extract] [--no-units]` | create an instance from a spec file and add its sources. `--no-extract` skips the seed documents, `--no-units` installs no systemd units | `created`, `name-taken`, `connection-missing`, `seed-refused` |
| `cortex update <name> --spec <file> [--no-units]` | replace the instance's spec: sources, model and serve settings. New sources are added and every source's timer is (re)installed; the seed cannot change | `updated`, `seed-change-refused`, `not-active`, `no-such-instance` |
| `cortex remove <name>` | stop and delete the instance's timers and viewer; its directory and store stay | `removed`, `wrong-state`, `no-such-instance` |
| `cortex list` | instances: name, state, description, model, EKR version, viewer address | |
| `cortex mcp-line <name>` | print the `claude mcp add` line that serves the instance's store through `ekr mcp` | |

`cortex update` refuses a spec whose `name` is not `<name>`.

## Sources

A source id is `<instance>/<source>`.

| command | does | outcomes |
|---|---|---|
| `cortex run <instance>/<source> [--record-failure]` | run one source once. With `--record-failure`, which the timers pass, a failed run is also counted | `ran`, `fetch-failed`, `extraction-failed`, `apply-refused`, `disabled`, `no-such-source` |
| `cortex source enable <instance>/<source>` | enable a source that two failed runs in a row disabled, and its timer | `enabled`, `wrong-state`, `no-such-source` |
| `cortex source record-failure <instance>/<source> --reason <text>` | count one failed run; the second in a row disables the source and its timer | `counted`, `disabled`, `already-disabled`, `no-such-source` |
| `cortex source add --instance-name <name> --source-id <id> --name <name> --kind <Web\|Connectors\|Files> --schedule <calendar>` | register a source in the registry; `create` and `update` do this for the spec's sources | `added` |
| `cortex sources` | sources: id, kind, state, schedule, runs, consecutive failures | |

A `ran` line's detail carries `documents_new`, `documents_applied`, `cost_usd`, `facts_refused`
(facts the model cited unissued evidence for), `parts_rejected` (parts EKR rejected), `masked`
(credential shapes replaced) and `stopped` (why the run ended early, if it did).

## Setup

| command | does |
|---|---|
| `cortex setup [--ekr-version <v>]` | install `ekr` at the version (default 0.0.30) with `cargo install` when it is missing |
| `cortex schema` | print the spec file's JSON Schema |
