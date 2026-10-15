---
format: aep.planning-md/3
id: story:next-link-paging
kind: story
status: draft
title: A paged source follows a next link and stops at the run's since
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A paged source can follow a next link: the `NextLink` paging style reads the cursor for the next
page out of a URL in the page body (`_links.next: "/api/pages?cursor=abc&limit=50"` asks next for
the input `cursor: abc`). An optional stop rule, `stop_older_than_since: true`, ends the walk at the
first record whose `time` is older than the run's `{since}`, so an operation that lists
newest-modified first is read down to the last run and no further. Example, on a `kind:
connectors` source:

```yaml
operation: pages.changed
records: results
id: id
time: version.createdAt
paging: {style: NextLink, param: cursor, next: _links.next, max_pages: 20, stop_older_than_since: true}
```

## Why

A downstream consumer runs cortex at a 10-minute cadence against an operation that lists
newest-modified first and has no `updated_after` input. Today such a source is read one page per
run, so a burst of changes bigger than one page between two runs is lost. Verified in this tree:

- `cortex.instance.PageStyle` has the variants `PageNumber`, `Token` and `Keyset` only, and
  `cortex.instance.Paging` the fields `style`, `param`, `next` and `max_pages`
  (`spec/domains/instance.yaml:105-115`). `Paging` is used by `ConnectorsSource` (`:136`),
  `ChildCall` (`:123`), `StructuredChild` (`:199`), `CompareLink` (`:210`) and
  `StructuredConnectors` (`:224`).
- With `next`, the walk takes the value at that path as the next `param` as it is, whatever the
  style (`src/sources.rs:177-201`, `next_page`). A next link would be sent whole as the cursor
  (`cursor: "/api/pages?cursor=abc&limit=50"`); nothing parses a URL.
- The walk ends at an empty page, a missing `next`, a page already asked for, or `max_pages`
  (`src/sources.rs:203-259`, `walk`); nothing in it reads a record's time, so it cannot stop at the
  window's start. The style is mapped by index (`src/model_map.rs:170-185`, `V0`..`V2`).
- The window's `{since}` is computed per run (`src/sources.rs:64-101`, `Window::of_run`;
  `src/run.rs:166-172`); a walk that reaches `max_pages` with pages left adds an `unread` line and
  the run keeps its `{since}` as `pending_since` (`src/run.rs:284-290`).
- The paging rules are documented at `website/docs/spec-file.md:172-177`.

## Domain relations

No new noun. `NextLink` is a variant of `cortex.instance.PageStyle` and `stop_older_than_since` a
field of `cortex.instance.Paging` (`spec/domains/instance.yaml:105-115`); both are values of a
`Source`'s settings, owned by `cortex.instance.Instance` through its `sources` relation.

## Work

- Spec first: in `spec/domains/instance.yaml`, add `NextLink` to `cortex.instance.PageStyle` and an
  optional Boolean `stop_older_than_since` to `cortex.instance.Paging`, with comments stating the
  rules below; run `ess specify validate` and the conformance suite green, then `task generate`.
  Only then change the code.
- `src/model_map.rs`: map the new variant and field. A spec with `stop_older_than_since: true` on a
  paging whose source has no `time` path (a child, a link, a `structured` input, or a `connectors`
  source without `time`) is refused at load, naming the field.
- `src/sources.rs` (`next_page`, `walk`): for `NextLink`, read the string at `next`, accept a
  relative or absolute URL, and take the percent-decoded value of the query parameter named by the
  last segment of `param` as the next `param`. A link that is missing, null or empty ends the walk;
  a link without that parameter ends it and adds an `unread` line naming the operation. The
  existing already-asked and `max_pages` guards apply unchanged.
- The stop rule: with `stop_older_than_since: true`, the walk reads each record's `time` (as
  `instant` reads it, `src/sources.rs:617`) and ends at the first record older than the window's
  `since_ms`; that record and the ones after it on the page are not taken. A walk ended by the
  rule read everything the window asks for and adds no `unread` line. A record with no readable
  time does not stop the walk. Reaching `max_pages` before the rule fires keeps today's `unread`
  line, so the run holds its window.
- Unit tests in `src/sources.rs` beside `max_pages_tests` (`:1937`), and one end-to-end case in
  `tests/e2e.rs` with the stand-in `connectors` answering three linked pages.
- Document `NextLink` and the stop rule in `website/docs/spec-file.md` (the `paging` paragraph).

## Acceptance

- `ess specify validate` passes with `NextLink` in `PageStyle` and `stop_older_than_since` in
  `Paging`, and `task drift` is clean after `task generate`.
- A `NextLink` walk over a stand-in answering `_links.next: "/api/pages?cursor=abc&limit=50"`
  invokes the second page with input `cursor: "abc"`, and an absolute link
  `https://host/api/pages?limit=50&cursor=a%2Fb` gives `cursor: "a/b"`.
- With `stop_older_than_since: true`, a walk over three pages whose second page holds the first
  record older than `{since}` invokes no third page, returns only the records before that one, and
  the run's report has no `unread` line; the next run's `{since}` moves forward as for a
  successful run.
- With `max_pages: 2` and the older record on page 3, the run reports
  `max_pages 2 reached with pages left` and keeps its `{since}` as `pending_since`.
- A spec with `stop_older_than_since: true` on a `connectors` source without `time` is refused at
  load with a message naming `paging.stop_older_than_since`.

## Not in this story

Paging styles other than a cursor in a next-link query parameter (a link header, a POST body);
a stop rule on child calls, links or `structured` inputs; any change to the window, the overlap or
seen-state.
