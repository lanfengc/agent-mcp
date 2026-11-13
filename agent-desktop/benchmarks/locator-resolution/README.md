# Locator resolution benchmark

A deterministic harness that compares the legacy full-snapshot locator path with the handle-free observed-tree evaluator. Both run over synthetic Chromium and Electron accessibility trees. The trees cover deep anonymous wrapper chains, duplicate role and name candidates, moving bounds, `AutomationId` and `DOMAutomationId` set at once, containment predicates and large trees.

Run it without network access:

```powershell
cargo run -p agent-desktop-core --release --example locator_benchmark |
  Set-Content "$env:TEMP\agent-desktop-locator-synthetic.json"
```

The JSON report contains:

- p50 and p95 wall-clock latency over 31 measured runs
- Candidate nodes, predicate work and the synthetic attribute values requested
- Cardinality and correctness

The three live modes measure different things:

- `live_find_selected_refs` uses the same selected-match-only materialization as the default CLI `find` path.
- `live_arena_direct` is a lower-overhead direct-target metric, reported separately.
- `live_count_no_refmap` checks that count-only requests perform no action or settable-evidence reads.

The harness does not report absolute native UIA call counts, because role-dependent action and settable probes make that count inaccurate. Live Slack and Electron measurements need those apps installed and unlocked. Run them with the opt-in Windows live harness under `tests/e2e-windows/`, not with this benchmark.

To produce the repo-wide report, run the probe A/B harness (`scripts/perf_ab_probe_test.py`). The report covers the HEAD vs merge-base fixture A/B and this benchmark.

## Electron and Chromium activation (Windows)

Chromium and Electron apps publish their UI Automation tree when a client asks for it. The Windows adapter reads the tree through UI Automation; `snapshot --force-electron-a11y` is available for a process that exposes nothing until accessibility is forced. The benchmark reads standards-based trees and makes no activation claim.
