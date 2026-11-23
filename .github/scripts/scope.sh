#!/bin/sh
# Reads the paths a pull request changes, one per line, on standard input, and prints `skip` when
# every path is planning or prose, else `run`. An empty list prints `run`.
#
# Planning: `.engineering/**`. Prose: `*.md`, except under the directories whose Markdown a check
# reads or generates: `website/` (the docs check), `spec/` (`compat-ack.md`), `generated/` (drift),
# and `src/`, `crates/`, `tests/`, `examples/`.
#
# `tests/it/ci_scope.rs` holds both sides of this list.
set -eu

seen=0
while IFS= read -r path || [ -n "$path" ]; do
  [ -n "$path" ] || continue
  seen=1
  case "$path" in
    .engineering/*) ;;
    website/* | spec/* | generated/* | src/* | crates/* | tests/* | examples/*)
      echo run
      exit 0
      ;;
    *.md) ;;
    *)
      echo run
      exit 0
      ;;
  esac
done

if [ "$seen" -eq 1 ]; then
  echo skip
else
  echo run
fi
