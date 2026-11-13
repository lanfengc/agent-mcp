# JSON output contract

Every command returns structured JSON:

```json
{
  "version": "2.6",
  "ok": true,
  "command": "click",
  "data": {
    "action": "click",
    "disposition": { "delivery": "delivered_unverified", "retry": "unsafe" }
  }
}
```

Errors include machine-readable codes and recovery hints:

```json
{
  "version": "2.6",
  "ok": false,
  "command": "click",
  "error": {
    "code": "STALE_REF",
    "message": "@s8f3k2p9:e7 not found in current RefMap",
    "suggestion": "Re-run a snapshot to obtain fresh refs, then retry with the new ref (CLI: snapshot [--skeleton]; FFI: ad_snapshot then pass the printed qualified ref to ad_execute_by_ref)",
    "recovery": {
      "strategy": "refresh_snapshot_then_retry_original",
      "retryable": true,
      "requires_fresh_snapshot": true
    },
    "disposition": {
      "delivery": "not_delivered",
      "retry": "safe"
    }
  }
}
```

`error.code`, `error.message` and `error.disposition` are always present. `suggestion`, `recovery`, `platform_detail` and `details` are omitted when empty. `recovery` appears only when `disposition.retry` is `safe`, and its `retry_after_ms` is set only for `TIMEOUT` and `APP_UNRESPONSIVE`.

Version 2.2 replaces the 2.1 `TIMEOUT` error for a budget-exhausted snapshot
with a successful, explicitly partial response, and adds `data.complete`.
Snapshot data always carries `complete`. A snapshot that runs out of budget now
returns `ok: true` with `"complete": false`, the tree it did observe,
`"truncated": true`, and `nodes_observed`, instead of a `TIMEOUT` error and no
tree. Every node whose descendants were cut short carries
`"subtree_truncated": true`, so a reader can walk from the root to each
boundary. Consumers must read `data.complete` to decide whether a tree is whole;
a `TIMEOUT` error no longer signals an oversized tree. A `--root` drill-down is
unaffected: it replaces refs inside an existing snapshot, so an incomplete
observation still returns `TIMEOUT` rather than a partial tree.

Version 2.1 replaces the 2.0 `error.retry_command` string with the structured
`error.recovery` object and adds `error.disposition`. Consumers must select a
recovery strategy from `recovery.strategy` only when `disposition.retry` is
`safe`; command strings from older envelopes must not be executed blindly.
The removed `retry_command` field has no compatibility alias.

## Sandbox results

`sandbox install` returns `data: {"providers": [...]}` and `sandbox list` returns `data: {"sandboxes": [...]}`. An entry has `name`, `state`, and `message`, with `next` when a further action is available; a sandbox entry, and the `data` of a single sandbox command, also carry `provider`. Provider-specific fields may appear beside these. `sandbox install <provider>` returns that entry as `data`. A build that includes no providers returns an empty `providers` array, and a command that names a provider fails with `INVALID_ARGS`. States are `unavailable`, `missing`, `ready`, `working`, `needs_setup`, `running`, `stopped`, and `broken`.

A sandbox whose record is missing, malformed or unreadable, or names a provider this build does not include, is listed as `broken`, and every command on it returns `INVALID_ARGS`. agent-desktop has no provider to ask, so `next` and the error's `suggestion` name the sandbox directory to delete once nothing in it is still running. When the provider's own status check fails or times out, the entry is also `broken` and `next` carries that error's suggestion.

A `sandbox create` that times out, or whose provider ends without a readable reply, keeps its record, because the provider may have made something. The sandbox stays in `sandbox list`; run `sandbox status <name>` to see what exists and `sandbox rm <name>` before creating that name again. A create the provider refuses leaves no record unless the provider left files in the sandbox directory. `create` never takes over a directory that already holds files.

`create`, `start`, `stop` and `rm` on one sandbox run one at a time. A second one fails at once with `ACTION_FAILED` and `not_delivered`.

`sandbox exec` passes the guest command's stdout and exit status through unchanged instead of adding an envelope; the guest's stderr is not passed through and the guest receives no stdin, so a guest command that reads `--stdin` gets empty input. A provider failure returns a host error envelope instead. agent-desktop reads at most 8 MiB from each of the guest's output streams; a command that prints more fails with `ACTION_FAILED`, so write large output to a file in the guest. A guest command that runs longer than 300 seconds fails with `TIMEOUT`. The host's session and agent id never reach the guest: the flags are refused before `sandbox exec`, and `AGENT_DESKTOP_SESSION` and `AGENT_DESKTOP_AGENT_ID` are not passed on. Give the guest command its own `--session` after `--`.

## Disposition values

Every error carries `disposition`, and commands that act on a target may also
carry it on their result. `delivery` is one of `unknown`, `not_delivered`,
`delivery_uncertain`, `delivered_unverified` or `delivered_verified`. `retry`
is one of `unknown`, `safe` or `unsafe`, and it follows from `delivery`:
`not_delivered` is `safe`, and every other non-`unknown` value is `unsafe`.

`unknown` means the command established nothing about delivery. A failure of a
mutating command that carries no delivery evidence, such as a `TIMEOUT` on an
operation that may already have landed, keeps it. A read-only command sends
nothing and invalid arguments never reach the application, so their errors, and
a lookup failure such as `APP_NOT_FOUND`, carry `not_delivered`. Treat `unknown`
as not safe to retry automatically: read `error.code` and the message, and
choose a recovery strategy only when `disposition.retry` is `safe`.

## Version 2.4 migration

Stateful ref actions (`set-value`, `type`, `clear`, `check`, `uncheck`, `toggle`, `expand`, and `collapse`) now reject observed postcondition contradictions after one delivery attempt. Readback failures preserve the adapter's error code and carry `details.kind: "post_action_verification"`. Missing evidence alone remains a successful `delivered_unverified` result with `details.verification_scope: "unavailable"`; neither that result nor verified element state proves application-level persistence.

Windows mutation errors that do not prove non-delivery stop fallback chains with an unsafe retry disposition. This affects activation, selection, disclosure, scrolling, and window operations; an unchanged or unreadable state is not proof that a mutation was never delivered. Value-writing chains also stop after unverified delivery instead of applying a second mechanism. Failures after preparatory scrolling carry `details.preparatory_scroll`, which records that the view may have changed. Their retry disposition follows the failure: a refusal that sent nothing is `not_delivered` and safe to retry, and any other failure keeps its own disposition.

Inspect `disposition.retry` before retrying or choosing an alternative action, regardless of `error.code`. An error may include `post_state` and `postcondition_satisfied` without erasing the original execution failure. The same verification rules apply to FFI ref actions; the C ABI is unchanged. The Rust `ActionResult::from_execution` constructor now takes only action and steps and returns an infallible result; live verification belongs to `execute_verified_action`.

## Version 2.5 migration

Version 2.5 is a breaking payload change for the commands that dispatch pointer, drag, clipboard, app-close and window operations: `mouse-click`, `mouse-move`, `mouse-wheel`, `hover`, `drag`, `clipboard-set`, `clipboard-clear`, `close-app`, `move-window`, `resize-window`, `minimize`, `maximize` and `restore`.

The unconditional success flags are removed with no compatibility alias: `clicked`, `moved`, `scrolled`, `hovered`, `dragged`, `resized`, `minimized`, `maximized`, `restored`, `closed`, `cleared`, the `requested` field of `close-app`, and the inner `ok` of `clipboard-set`. A flag that every successful call carried proved nothing about the operation.

A verified window report names the exact readback in `details.verification_scope` (`window_size`, `window_position`, `window_minimized_state` or `window_maximized_state`). Geometry scopes also state `details.tolerance` and `details.units`: 8 px on Windows. A window failure after the write keeps its code, details and disposition and gains `details.target` and the completed `details.steps`.

Each of these commands now returns the adapter's own report inside `data`:

```json
{
  "action": "drag",
  "disposition": { "delivery": "delivered_unverified", "retry": "unsafe" },
  "steps": [ { "label": "pointer_drag", "outcome": "succeeded", "mechanism": "physical_synthetic" } ],
  "target": { "kind": "drag", "from": { "x": 10, "y": 20 }, "to": { "x": 80, "y": 20 } },
  "from": { "x": 10, "y": 20 },
  "to": { "x": 80, "y": 20 }
}
```

- `disposition.delivery` is `delivered_verified` only after the adapter read back the postcondition it names, `delivered_unverified` after a completed dispatch without that proof, and `not_delivered` only when the adapter observed that the operation was already satisfied (for example `close-app` on an application that had already exited). `ok: true` means the command returned a delivered or already-satisfied result, not that the application effect was verified.
- `target.kind` is `window` (`id`, `title`, `process`), `app` (`name`, `process`), `pointer` (`point`), `drag` (`from`, `to`) or `clipboard`. The adapter supplies the identity it validated and used. A pointer target is the dispatch coordinate and does not prove which application received the input.
- Request context (`x`, `y`, `count`, `dx`, `dy`, `width`, `height`, drag endpoints, `drop_delay_ms`, `type`, `app`, `method`, and a confirmed `focused`) keeps its earlier meaning. These fields echo the request and are not observed post-state.
- Errors keep `error.disposition`. A failed `--wait-for` after a delivered operation keeps the operation's own delivery (verified, unverified, or not delivered) in `error.disposition` and carries the report under `details.after_action`.
- The Rust `ActionResult` and `OperationReport` readers reject a payload without an explicit `disposition`; a missing disposition no longer reads back as delivered-unverified.
- The C ABI is unchanged. Status-only functions such as `ad_close_app` and `ad_drag` still cannot expose the disposition, steps or target of a successful call.

## Version 2.6 migration

Version 2.6 changes which refs resolve. Such a ref resolves only when one complete read of its window proved it unique when it was captured and exactly one live element matches its role and stable fields now. The only new payload fields are `data.identity_unproven` and, on a ref action whose budget ran out after a refusal, `error.details.refusal`.

- A ref to one of two identical controls, which earlier resolved by position or geometry, now fails `AMBIGUOUS_TARGET` with `details.kind: "identity_not_unique"` before anything is read. So does a ref captured from a window that could not be read whole within the snapshot timeout.
- A refmap written before 2.6 records no capture proof, so all of its refs fail the same way. Take a new snapshot.
- An identical twin that appears after capture fails `AMBIGUOUS_TARGET` with `details.kind: "saved_ref_ambiguous"`.
- All of these carry `details.retryable: false` and a `not_delivered` disposition. `disposition.retry` stays `safe` because nothing was sent, but repeating the same ref returns the same refusal.
- When a ref action's full resolution under the lease is refused in a way that may clear, the action waits again within its `--timeout-ms`. If that budget then runs out, the error carries the refusal in `details.refusal`: its `code`, `message`, and `details` when the refusal had any.
- A surface kind the OS does not have now returns `INVALID_ARGS` instead of `PLATFORM_NOT_SUPPORTED`, with the disposition `not_delivered` and `retry: "safe"`. FFI callers see the matching `AdResult` code change. An adapter with no surfaces at all still returns `PLATFORM_NOT_SUPPORTED`.

A snapshot that needed one complete read of its window to prove its refs, and could not get it, says so with `"identity_unproven": true` in `data`. `complete` still describes the presented tree. Every ref of that snapshot fails `AMBIGUOUS_TARGET` with `details.kind: "identity_not_unique"`; retake the snapshot with a larger `--timeout-ms`. The field is omitted otherwise.

- Focus and preparatory scrolling can change coordinates. Stability is verified against fresh live observations, not by requiring coordinates to remain equal to the original snapshot.
- Focusing an inactive text field can change its selection. A pre-focus range cannot establish the expected insertion into a nonempty field; such a result is explicitly unverified rather than falsely contradicted.
- A settle-loop pacing budget does not replace the caller's deadline for native reads. Tests assert deadline identity rather than arbitrary remaining-time thresholds.

### Post-action observation

When a ref action returns `delivery_uncertain` with a post-action wait selector, `error.details.post_action_observation` records a read-only observation of the original process. Application-owned refs keep their source surface. Window-owned refs observe their original pinned window, including refs captured from `sheet`, `alert`, `popover`, `focused`, or a window-owned `menu`. This lets a closed overlay be observed without treating the parent window as absent. The action is never repeated, and its original error and unsafe retry disposition stay unchanged. Even `expected_state_observed` does not prove that the action caused the state. Observation uses `--wait-timeout` separately from the action timeout.

| Field | Meaning |
| --- | --- |
| `kind` | Always `post_action_observation`. |
| `selector, expected` | Requested selector and `present` or `absent`. |
| `surface` | The observed scope: the application-owned source surface, or `window` for a window-owned ref. |
| `process_identity` | Original `pid` and `process_instance`; the instance can be null for `identity_unavailable`. |
| `elapsed_ms` | Time spent observing after the action returned. |
| `source_window_id` | Original window ID, when the ref has one. |
| `selector_matches` | Observed selector result, when available; absence requires a complete read. |
| `process_state` | Last observed `unresponsive`, `exited`, or `crashed` state. `running` is never emitted. |
| `last_observation_error` | Last observation error's `code` and `message`, when available. |
| `outcome` | One of the outcomes below. |

| Outcome | Meaning |
| --- | --- |
| `expected_state_observed` | The expected selector state was observed, or the original window or surface was confirmed absent for an absence wait. |
| `process_exited` | The original process exited; this does not establish the action's effect. |
| `process_crashed` | The platform reported that the original process crashed. |
| `owner_unavailable` | The original surface owner remained unavailable without confirmed process exit. |
| `observation_failed` | A terminal read error stopped observation; inspect `last_observation_error`. |
| `timed_out` | The wait budget expired without observing the expected state, including when transient read errors persisted through the last poll. `last_observation_error` may hold the last error. |
| `identity_unavailable` | The ref has no usable process generation, so observation was skipped. |

## Background click and capture metadata

A screenshot of one window (`--window-id` or `--app`) adds `data.capture_id`, `data.window_id`, `data.window_bounds`, `data.capture_bounds` and `data.capture_time_ms` when the window has bounds. `capture_bounds` is the screen rectangle that the image pixels cover. On Windows it is the DWM extended frame when the image has that size, and `window_bounds` otherwise. `capture_id` is opaque; pass it unchanged to `mouse-click --background` to make `--xy` image pixels.

`screenshot --ocr` (Windows) adds `data.ocr`: `text` and `lines`, where each line has `text`, `rect` and `words`, and each word has `text` and `rect`. A `rect` is `x`, `y`, `width` and `height` in image pixels. `--ocr-query` adds `data.ocr_match` with `text`, `line`, `x`, `y`, `width`, `height`, `center_x` and `center_y`. The field is omitted when nothing matches.

`mouse-click --background` (Windows) returns the adapter report with the request echoed beside it: `window_id`, `space` (`screen` or `image`), `screen_x`, `screen_y`, `restore_minimized`, `cursor_fallback` and `semantic`, plus `capture_id`, `image_x` and `image_y` when a capture id was passed. `details` depends on how the click was delivered:

| `details.method` | Other `details` fields |
| --- | --- |
| `PostMessage` | `background: true`, `target_hwnd`, `client_x`, `client_y`, `window_was_minimized`, `restored_for_click`, `cursor_fallback`, `semantic_requested` |
| `PostMessage+cursor` | The same fields; the cursor was aimed at the point for the click and put back. |
| `UIA.Invoke` | `background: true`, `count`, `semantic: true`. No window or client fields. |

`target_hwnd` is the window that received the messages, as a window id: the child control under the point when there is one, otherwise the target window. `client_x` and `client_y` are in that window's client coordinates. The disposition is always `delivered_unverified` with `retry: "unsafe"`: the messages or the invoke call were accepted, not proven to have changed the app.

A capture that no longer matches the window, its process instance, its bounds or the capture-age limit (default 10 seconds, `AGENT_DESKTOP_MAX_CAPTURE_AGE_MS`) fails with `STALE_REF`, `not_delivered`, and `details.kind: "stale_capture"`, `details.reason` and `details.capture_id`.

## Find result shape

`find` match objects report the element's actual accessibility `name`. Elements
without an accessible name omit `name` entirely; their content stays in `value`.
Earlier output synthesized a display name, falling back to `value` or
`description`, then a `(unnamed <role>)` placeholder, so consumers that treated
`name` as always present must tolerate its absence and apply their own fallback
to `role` or `value`.

## Ref re-identification details

Every ref is re-identified from its stored entry or refused, and `details.kind` names which. The core resolver refuses a saved ref with one of these three kinds:

| `details.kind` | Code | Meaning |
|------|------|---------|
| `saved_ref_absent` | `STALE_REF` | No live element matches the stored role and stable fields over complete evidence, so the element is gone. Take a fresh snapshot. |
| `saved_ref_ambiguous` | `AMBIGUOUS_TARGET` | An element identical to the captured one appeared, so two or more live elements match the stored role and stable fields. A fresh snapshot refuses both as twins; act on a control that is unique in the window. |
| `identity_not_unique` | `AMBIGUOUS_TARGET` | The ref was not proven the only element matching its role and stable fields when it was captured, so nothing is searched. Either an identical twin existed, or the window could not be read whole within the snapshot timeout (retake the snapshot with a larger `--timeout-ms`). A twin is refused by every snapshot; only an explicit coordinate click (`--headed mouse-click --xy`) can pick one, and it takes the foreground. |

The fourth outcome is `resolution_identity_unknown`, which carries `TIMEOUT`: a live field could not be read, so the identity count stayed unproven. It is retryable.

After the core resolver chooses the element, the Windows adapter lands it, and a landing failure keeps the adapter's own `details.kind` (for example `resolve_no_candidate`). The platform skills list those kinds. `saved_ref_absent`, `saved_ref_ambiguous` and `identity_not_unique` carry `details.retryable: false`, because waiting cannot change the answer. Every one of these is `not_delivered`.

## Error codes

| Code | Meaning |
|------|---------|
| `PERM_DENIED` | The operation needs a permission the agent does not have |
| `ELEMENT_NOT_FOUND` | No element matched the ref or query |
| `APP_NOT_FOUND` | No matching application process; the lookup failed before anything was sent, so the disposition is `not_delivered` |
| `STALE_REF` | Ref could not be re-identified in the live UI |
| `AMBIGUOUS_TARGET` | Ref recovery matched multiple plausible targets |
| `SNAPSHOT_NOT_FOUND` | Snapshot ID is missing or expired |
| `SANDBOX_NOT_FOUND` | No sandbox has that name; run `agent-desktop sandbox list` to see the sandboxes that exist |
| `POLICY_DENIED` | Physical/headed path blocked by policy |
| `ACTION_FAILED` | Action rejected, or its observed result contradicted the request |
| `ACTION_NOT_SUPPORTED` | The target does not expose the requested action |
| `APP_UNRESPONSIVE` | The matching application stopped responding |
| `PLATFORM_NOT_SUPPORTED` | Adapter method not implemented on this platform |
| `TIMEOUT` | Wait condition expired, or a live identity read stayed incomplete until the deadline |
| `INVALID_ARGS` | Invalid argument values |
| `WINDOW_NOT_FOUND` | The requested window does not exist, including a `screenshot --window-id` that matches no window; the disposition is `not_delivered` |
| `NOTIFICATION_NOT_FOUND` | The notification ID was not found |
| `INTERNAL` | Unexpected error or caught panic |

## Exit codes

`0` success, `1` structured error (JSON on stdout), `2` command-line parse error (the envelope is still printed, with `INVALID_ARGS`).
