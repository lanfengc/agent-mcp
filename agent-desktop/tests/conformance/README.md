# Adapter reliability conformance

The adapter must meet the same ref and action contract on every surface it reads. The tests are written against `PlatformAdapter` semantics, so window, menu and system surfaces share one set of expectations.

The reusable command-path helper is `tests/conformance/ref_action_contract.rs`. `window_identity_contract.rs` covers window identity. `src/tests/conformance.rs` mounts both and runs the ref and action helper against a mock adapter. A fixture that drives the real adapter can call the same helper with adapter-provided refs.

## Required gates

| Area | Required behavior |
|------|-------------------|
| Snapshot refs | Refs are depth-first, snapshot-qualified (`@<snapshot_id>:eN`), and a bare `@eN` is rejected with `INVALID_ARGS` |
| Strict resolve | A ref resolves only when identity still matches; stale refs return `STALE_REF` |
| Ambiguity | Multiple plausible matches return `AMBIGUOUS_TARGET`, never an arbitrary click |
| Actionability | Ref actions check live visibility, stability, enabled state, supported action, policy, and editability before dispatch |
| Wait recovery | `wait --element` polls the snapshot embedded in its qualified ref, honors the caller timeout while resolving, and reports the last observed predicate state |
| Snapshot scope | A qualified ref resolves its own snapshot inside the selected session namespace and never searches other sessions |
| Trace | With a `trace: on` session manifest (from `session start`), commands write per-process JSONL segments under the session directory. `--trace <path>` overrides to one file. Best-effort unless `--trace-strict`. |
| Session lifecycle | `session start/end/list/gc` manage manifests, the current-session pointer, and trace directories |
| FFI parity | FFI ref actions use strict resolve and actionability checks before adapter dispatch |

## Surface matrix

| Fixture | Windows UIA |
|---------|:-----------:|
| Two identical buttons produce ambiguity | Required |
| Ref disappears after snapshot | Required |
| Disabled button blocks click before dispatch | Required |
| Text field supports value/type actionability | Required |
| Session A latest refmap is invisible to Session B | Required |
| Batch item session overrides inherited session | Required |
| FFI `AdRefEntry` preserves full identity envelope | Required |

## Adding a fixture

Add adapter-specific integration fixtures, but keep expected errors and JSON shapes identical. Prefer semantic UIA patterns first (Invoke, Value and Selection patterns). Coordinate input is a lower-confidence fallback, so it must stay explicit in policy or command choice.

## Windows private-storage gate

Verify Windows private-file behavior with native tests. Cross-compilation alone is not evidence. The Windows CI lanes run these suites natively:

| Behavior | Native test |
|----------|-------------|
| A junction as a path component, the parent, or the leaf is refused and nothing lands at its target | `crates/windows/src/system/private_file/path_tests.rs` |
| A validated ancestor cannot be renamed while the write holds its no-follow handle | `a_pinned_directory_cannot_be_renamed_while_its_no_follow_handle_is_held` in `path_tests.rs` |
| A foreign owner is refused on atomic write, append, and bounded read | `owner_tests.rs` |
| A fresh artifact keeps its inherited SYSTEM, Administrators, and user grants, and never `BUILTIN\Users`, across create and replace | `a_profile_artifact_keeps_system_admins_and_user_and_never_users_across_create_and_replace` in `owner_tests.rs` |
| Remote or undeterminable storage refuses the write and append surfaces | `locality_tests.rs` |
| Atomic replace succeeds over open readers and concurrent writers, and leaves no temporary residue | `replace_tests.rs` |
| The binary and the FFI adapter install the hardened seam, and `session start` through a junction state root is refused with nothing written under the target | `src/tests/windows_private_file_install.rs`, `crates/ffi/tests/c_abi_windows_private_file_install.rs` |

Each private directory the module creates gets a protected DACL (`dacl::protect_directory`) that grants SYSTEM, Administrators, the token user and the owner-eligible SIDs. `dacl::require_private` checks that DACL before use. Files created in such a directory inherit those grants. The test `a_profile_artifact_keeps_system_admins_and_user_and_never_users_across_create_and_replace` in `owner_tests.rs` pins this.
