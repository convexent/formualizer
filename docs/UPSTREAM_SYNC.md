# Upstream Sync (convexent fork ↔ psu3d0/formualizer)

This repository (`convexent/formualizer`) is a fork of upstream `psu3d0/formualizer`.
This document records the divergence audit and the policy for keeping the fork
in sync. It is referenced by the supermod ExecPlan for issue
`convexent/supermod#2148`.

## Audit — divergence as of 2026-06-18 (historical; see the 2026-09-21 sync below)

Measured against `upstream/main` and `origin/main` (fetch both first):

    git fetch upstream main && git fetch origin main
    git merge-base origin/main upstream/main      # 4d2aaf4066b65b67edaf75bf0399a6e703f3ff89
    git rev-list --count origin/main..upstream/main   # 191  (upstream-only commits)
    git rev-list --count upstream/main..origin/main   # 10   (fork-only commits)

- **Merge-base:** `4d2aaf4`.
- **Fork is 191 commits behind / 10 ahead.** Fork `main` is version `0.5.11`;
  upstream `main` is `0.7.0` — **two minor versions** of drift.

### What upstream has that the fork does not (headline architecture)

Upstream's lead is not only bug-fixes; it includes major, behaviour-affecting
architecture that a fork sync (and any supermod re-pin) must re-validate:

- **FormulaPlane** — adaptive formula partition unifying legacy + FormulaPlane
  evaluation (upstream #142–#148).
- **Iterative calculation** — `CyclePolicy::Iterate`, Excel-style iterative
  calc, and runtime cycle detection via live-edge SCC iteration (upstream
  #112/#113 → #118/#119/#130).
- **Deferred-dirty scope** (#139) — batch edits run one multi-source dirty
  propagation (up to 270× on batch edits). The fork does **not** carry this;
  see the note in `crates/formualizer-eval/src/engine/graph/mod.rs`
  (`mark_dirty_many`).

### Fork-only commits (10) and their status

    git log --oneline upstream/main..origin/main

    965e5cd chore: remove build artifacts from repo + .gitignore them   (fork-only)
    3392ea8 fix(workbook): write formula cached values via set_formula_result_*  (fork-only)
    9401da8 Bump to v0.5.11                                              (fork release tooling)
    abd1dc1 fix(eval): preserve named-range edges through CSR rebuild …  (already upstreamed via #108)
    a98d4c0 Bump to v0.5.10                                              (fork release tooling)
    5501d95 fix(eval): walk through Named/Range pass-through vertices …  (already upstreamed via #108)
    acb78c8 Bump to v0.5.9                                               (fork release tooling)
    81401cc fix(eval): EDATE/EOMONTH year-boundary off-by-one …          (fork-only)
    39f8bdc fix(eval): route date-function coercion … (#17)              (already upstreamed via #107)
    5009dd4 fix(eval): drain all staged sheets … (#16)                   (already upstreamed via #106)

Upstream has already absorbed convexent's drain-staged-sheets (#106),
date-fn-coercion (#107), and named-range-passthrough (#108) PRs. The
genuinely fork-unique, not-yet-upstreamed deltas are small: the EDATE/EOMONTH
off-by-one fix, the `set_formula_result_*` cached-value write, the
build-artifact cleanup, and the fork's own version-bump release commits.

### #2148 changes (this branch)

- Ported upstream's `mark_dirty_many` into the fork (graph/mod.rs), replacing
  the interim inline `redirty_volatiles` fix from PR #20 — the
  convexent/supermod#2130 O(V·N) volatile-redirty quadratic. `mark_dirty`
  now delegates to it; the deferred-dirty guard (#139) is omitted as dead code.
- Ported the `dirty_propagation_visits` observability counter (standalone
  `u64`, unrelated to #139).
- Adopted upstream's clippy-clean form for the two pre-1.93 test files so
  CI (`clippy 1.93.0 -D warnings`) is green on `main`.
- Adopted upstream's deterministic visit-count tests
  (`mark_dirty_multi_source.rs`, minus its iterative-calc test which the fork
  can't compile yet) as the #2130 regression guard — the volatile redirty
  asserts ~one component walk (251 visits) vs the ≥10 000 quadratic, and the
  union-equivalence test pins `mark_dirty_many` == sequential single-source.
  PR #20's interim wall-clock guard was evaluated and dropped as strictly
  dominated (timing-sensitive 5s bound, ~44s setup per run, weaker signal), so
  nothing from PR #20 is carried — the fix and its tests all come from upstream.

## Sync — 2026-09-21 (`sync/2026-09-21`, fork 0.5.12 → upstream 0.9.3)

Merged `upstream/main` @ `362becff` (728 upstream-only commits; fork had 12).
Every fork-only engine commit now exists upstream (verified by matching each
fork commit to its upstream counterpart and diffing the patches — differences
were rustfmt/clippy style only):

    5009dd4 drain staged sheets        → upstream c54500ee (#106)
    39f8bdc date-fn coercion           → upstream eac698b2 (#107)
    81401cc EDATE/EOMONTH off-by-one    → upstream 5ae1613b
    5501d95 / abd1dc1 named pass-through → upstream 0d601a1d / 7d87e8e1 (#108)
    67bd5ef mark_dirty_many port        → upstream original (#139)

All fork test modules (`cross_sheet_named_range_first_cell`,
`date_function_duration_cells`, `demand_subgraph_named_range`,
`dn_range_xlsx_subgraph`, `edate_eomonth_engine`, `mark_dirty_multi_source`)
exist upstream, identical or as supersets, so conflicts in those files were
resolved to upstream.

**Remaining fork-local carries** (the whole `git diff upstream/main` after the
merge):

- `crates/formualizer-workbook/src/backends/umya_impl.rs` — the
  `set_formula_result_*` cached-value write (fork 3392ea8). Upstream moved the
  umya backend into `umya_impl.rs` (shared via `include!` by the umya 2.3 and
  umya 3.1 backends, #448) and still uses `set_value_*` + `set_formula_obj`,
  so the fix was re-ported there. **Note:** at the pinned umya revs
  (PSU3D0/umya-spreadsheet `4b64d65` for 2.3.2, crates.io 3.1.0) the two paths
  are behaviourally equivalent: both leave the same `CellValue` state and
  serialize identical XML, including for loaded files with stale `t="str"`
  caches. The originally reported openpyxl `str` symptom could not be
  reproduced against either path during this sync. The port is kept because it
  uses umya's purpose-built API and is behaviour-neutral. It is guarded by
  `formula_cached_values_are_saved_as_typed_values`, which asserts the saved
  sheet XML (upstream's `formula_cache_batch` tests accept string caches). If
  the next sync wants to minimise divergence, dropping this carry is safe as
  long as that test stays green.
- `.gitignore` — local wheel/`dist*` and `repro-*.py` ignores (fork 965e5cd).
- This file.

Version files were taken from upstream (`0.9.3`, parser track `3.1.2`).

## Sync policy

The fork should remain a **thin layer over upstream**, not an independent
divergent branch. Concretely:

1. **Upstream-first for new fixes.** Land convexent engine fixes as PRs against
   `psu3d0/formualizer` first (as #106/#107/#108 already were), then pick them
   up here on the next sync. Only carry a fork-local commit when upstream
   cannot or will not take it, and record why in this file.

2. **Cadence.** Run a sync audit at least **monthly**, and additionally
   whenever supermod is about to bump its `formualizer==` pin. The audit is the
   three `git` commands at the top of this file; if the behind-count has grown
   materially or upstream shipped a fix the fork has open-coded, open a sync.

3. **Mechanism (pull-only).** psu3d0 requires CI approval for upstream PRs, so
   syncing is one-directional: merge upstream into the fork, never the reverse
   automatically. Create a `sync/<yyyy-mm-dd>` branch, `git merge upstream/main`,
   resolve conflicts (favouring upstream for anything the fork had only
   open-coded), run the full `cargo test` + `clippy` gate, and open a PR. Do
   not rebase `main` onto upstream — a merge preserves the fork's own history
   and the release tags.

4. **Major-version syncs are projects, not chores.** A sync that crosses a
   minor version (e.g. the pending `0.5.x → 0.7.0`) pulls breaking architecture
   (FormulaPlane, iterative calc) and forces a supermod re-pin + full calc
   re-validation. Track those as their own issue/epic, not as a routine merge.
