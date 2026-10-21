# Status

Where cortex stands against its targets. Each row is dated (UTC) and names the command it was read
from; a row is true as of its date only. Commands and gates are in [AGENTS.md](AGENTS.md).

| target | state | read on | from |
|---|---|---|---|
| `task check` passes on `main` | pass (`e598c7e`) | 2026-10-13 | `gh run list --repo beyond10x/cortex --branch main --workflow check.yml --limit 1` |
| Documentation site serves `main` | pass (`e598c7e`) | 2026-10-13 | `curl -s https://beyond10x.github.io/cortex/.well-known/b10x-site.json` |
| open issues: at most 3 | pass (0) | 2026-10-13 | `gh issue list --repo beyond10x/cortex --state open` |
| latest release | `v0.2.2` | 2026-10-13 | `gh release list --repo beyond10x/cortex --json tagName,isLatest` |
| pins: ESS 0.60.0, aep 0.75.0, EKR 0.0.34 | pass | 2026-10-21 | `spec/ess-inputs.yaml`, `.github/workflows/check.yml`, `cortex setup --help` |
| `aep plan artifact validate` | pass | 2026-10-13 | `aep plan artifact validate` |
| 1.0: an instance runs 7 days unattended (`story:first-web-instance`) | running: `agent-tooling-4`, first timer 2026-10-15, read on 2026-10-22 | 2026-10-14 | `systemctl --user list-timers 'cortex-agent-tooling-4-*'` |
