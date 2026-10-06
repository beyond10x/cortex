# Hand-over: session brain-and-cortex, 2026-10-06

Written at the conductor's wrap-up dispatch DSP-20261006-13 (DEC-20261006-06). The session ends
after this file; the next owner starts from here.

## Open wave

**Wave 20261006i (cortex 1.0 wave 16)** — `specification:wave-20261006i-cortex-wave-16`, status
`approved`. One unit, U1, `story:postgres-credential-from-connectors`.

| branch | head | state |
|---|---|---|
| `wave/20261006i` | `efa6f90` | pushed 2026-10-06; holds the wave-open commit `30fd665` and the unit merge |
| `unit/postgres-credential-w16` | `8514d95` | merged into `wave/20261006i`; not pushed on its own (the wave branch holds it) |

The unit started a PostgreSQL store's `ekr` through a Connectors connection launch. Its gate passed
in the unit tree. What is left, in order:

1. Wait for epistemic-knowledge-runtime 0.0.32 (see that repository's hand-over: its release PR
   is red on browser-test races). Then move cortex's EKR pin from 0.0.31 to 0.0.32 on
   `wave/20261006i`.
2. Record "Design B" in `story:postgres-credential-from-connectors` with `aep plan artifact body`:
   the unit could not write under `.engineering/`, so its design note is only in the unit report.
3. One security review of the unit (it handles a PostgreSQL password handed over by a Connectors
   launch); record the result as a `review-result`.
4. `task check` on the wave branch, PR, close the wave (evidence, moves, Outcome), merge.
5. Next story on that path: `story:postgres-run-undo`.

The wave branch's `Taskfile.yml` still defaults `CARGO_TARGET_DIR` to a shared cache directory;
`main` no longer does (wave 17). Merging `main` into the wave branch before step 4 picks that up.

## Merged today

| PR | what |
|---|---|
| https://github.com/beyond10x/cortex/pull/48 | wave 20261006j: builds go into the tree's own `target/` (DSP-20261006-04, DEC-20261006-02) |

## Worktrees

`cortex-w16-a` and `cortex-w16-int` are finished with `--discard-cache --archive` after the wave
branch push. Recreate a tree from `wave/20261006i` to continue.

## Open pull requests

None from this session.

## Dispatches

| id | state |
|---|---|
| DSP-20261006-04 | done, reported (PR 48) |
| DSP-20261006-13 | this hand-over |
