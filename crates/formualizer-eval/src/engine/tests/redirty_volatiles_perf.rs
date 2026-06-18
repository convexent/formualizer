//! Regression guard for convexent/supermod#2130.
//!
//! `redirty_volatiles` used to call `mark_dirty` once per volatile vertex, and
//! `mark_dirty` walks the full transitive dependent cone with a fresh `visited`
//! set each time. When many volatiles share a large dependent cone, that is
//! O(V·N) and dominated evaluation on real workbooks (~219s of a 219s
//! `evaluate_cell`). The fix walks the union of cones once.
//!
//! Shape (chosen for O(V+N) construction — no high-fan-out vertex, which would
//! make graph-building itself O(N²)): V volatile `RAND()` cells feed one
//! intermediate `SUM`, which is the head of a length-N dependency CHAIN
//! (each cell references the previous). Every volatile's cone is therefore the
//! same ~N-vertex chain. Pre-fix, re-dirtying re-walks that chain once per
//! volatile = V·N visits; the batched-traversal fix walks it once = O(N). The
//! blowup is carried by V (cheap cells), keeping chain depth — and thus per-pass
//! evaluation — modest so the test stays fast post-fix.
//!
//! Measured (debug, `cargo test -p formualizer-eval`) at V=20000, N=2000, with
//! the fix reverted vs applied — i.e. the bound was watched go red:
//!   pre-fix  (per-volatile mark_dirty): second evaluate_all ~= 38.7s  -> FAILS
//!   post-fix (batched traversal):       second evaluate_all ~=  0.29s -> passes
//! The 5s bound therefore sits ~7.7x under the pre-fix time and ~17x over the
//! post-fix time. Each pre-fix cone-visit costs ~1us (a
//! `collect_range_dependents_for_vertex` allocation plus hash-set work), so
//! V*N ~= 40M lands at ~39s rather than the low single-digit seconds a cheaper
//! per-visit cost would imply. V is held at 20000 (not higher) to bound the
//! one-time ~22k-cell setup; the timed region itself is sub-second.

use super::common::create_cell_ref_ast;
use crate::builtins::random::register_builtins;
use crate::engine::{Engine, EvalConfig};
use crate::test_workbook::TestWorkbook;
use formualizer_parse::parser::{ASTNode, ASTNodeType};
use std::time::{Duration, Instant};

#[test]
fn redirty_volatiles_is_not_quadratic_in_volatiles() {
    register_builtins();
    let wb = TestWorkbook::new();
    let mut engine = Engine::new(wb, EvalConfig::default());

    const V: u32 = 20_000; // volatile sources (cheap; carries the V·N blowup)
    const N: u32 = 2_000; // chain depth (the cone each volatile re-walks)

    // Column 2, rows 1..=V: =RAND() (volatile). Laid out down a column (rows,
    // max ~1M) to stay within the 16384-column limit. Each has fan-out 1
    // (referenced only by the intermediate SUM), so construction stays O(V).
    let mut vol_refs = Vec::with_capacity(V as usize);
    for row in 1..=V {
        engine
            .set_cell_formula(
                "Sheet1",
                row,
                2,
                ASTNode {
                    node_type: ASTNodeType::Function {
                        name: "RAND".into(),
                        args: vec![],
                    },
                    source_token: None,
                    contains_volatile: true,
                },
            )
            .unwrap();
        vol_refs.push(create_cell_ref_ast(None, row, 2));
    }

    // Intermediate at (1,1) = SUM(all volatiles): a dependent of every volatile,
    // and the head of the chain below.
    engine
        .set_cell_formula(
            "Sheet1",
            1,
            1,
            ASTNode {
                node_type: ASTNodeType::Function {
                    name: "SUM".into(),
                    args: vol_refs,
                },
                source_token: None,
                contains_volatile: false,
            },
        )
        .unwrap();

    // Chain of length N down column 1: cell at row r references row r-1
    // (fan-out 1 each). (2,1)=R1C1 (the intermediate), (3,1)=R2C1, ... so the
    // whole chain is downstream of every volatile via the intermediate.
    for r in 2..(2 + N) {
        engine
            .set_cell_formula("Sheet1", r, 1, create_cell_ref_ast(None, r - 1, 1))
            .unwrap();
    }

    // Prime: first solve also re-dirties the volatile cone at the end.
    engine.evaluate_all().unwrap();

    // Timed: a second full solve re-evaluates the re-dirtied cone and calls
    // redirty_volatiles again — the operation that was O(V·N) before the fix.
    let t = Instant::now();
    engine.evaluate_all().unwrap();
    let elapsed = t.elapsed();
    eprintln!("[2130 guard] second evaluate_all (V={V}, N={N}): {elapsed:?}");

    assert!(
        elapsed < Duration::from_secs(5),
        "redirty_volatiles appears quadratic again: second evaluate_all took {elapsed:?} \
         for V={V} volatiles x N={N} chain (expected well under 5s after the \
         batched-traversal fix for convexent/supermod#2130)"
    );
}
