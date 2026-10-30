---
format: aep.planning-md/3
id: story:redaction-linear-in-links
kind: story
status: draft
title: Redaction time grows linearly with a document's link count
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

Redaction time grows linearly with the number of links in a document. Measured on the integration
branch, redaction alone took 5.5 / 46 / 613 ms for 512 / 2048 / 8192 links while the regex search
alone took 0.9 / 3.2 / 16 ms. The suspected cause is the overlap check in the scan
(`src/redact.rs` around `:1046`), unconfirmed. Pre-existing.

## Acceptance

- A measurement in the story's evidence: redaction of 2048 and 8192 links, median of 5 runs each,
  in the test profile; the 8192 median is at most 6x the 2048 median.
- 8192 links redact in at most 4x the regex search time for the same text.
- `bracketed_links_back_to_back_are_found_in_linear_time` passes 20 consecutive runs (one probe
  measured 8.8x against its 8x limit, a flake risk).

## Source

- Wave 20261030a coordinator review of the unit reports (the numbers above).
- `p2b` probe: `links_with_parentheses_without_a_partner_are_found_in_linear_time`, "4x the text
  took 9.8x the time (5.5 ms -> 54.4 ms)".
