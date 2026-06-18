# Upstream Sync (convexent fork ↔ psu3d0/formualizer)

This repository (`convexent/formualizer`) is a fork of upstream `psu3d0/formualizer`.
This document records the divergence audit and the policy for keeping the fork
in sync. It is referenced by the supermod ExecPlan for issue
`convexent/supermod#2148`.

## Audit — divergence as of 2026-06-18

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
- Salvaged the fork-new #2130 wall-clock regression guard
  (`redirty_volatiles_perf.rs`) and adopted upstream's deterministic
  visit-count tests (`mark_dirty_multi_source.rs`, minus its iterative-calc
  test which the fork can't compile yet).

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
