"""
Minimal repro: per-cell evaluate_cell pre-pass corrupts overlay state,
causing subsequent evaluate_all to return stale-mixed values.

Surfaced from supermod's formualizer_adapter.populate_cache pattern, which
walks defined-name cells in priority order calling evaluate_cell on each
before any evaluate_all. Worked in formualizer 0.4.6; broken in 0.5.x.

Run:
    python3 repro-pre-pass-overlay-bug.py
"""
from formualizer.formualizer_py import Workbook

WB_PATH = "repro-pre-pass-overlay-bug.xlsx"
TARGET = ("Model", 13, 5)  # base total_profit cell, formula:
                           # =SUM($E$11:$G$11, e2e_model_profit__upside,
                           #      e2e_model_profit__downside)


def reset_and_baseline():
    wb = Workbook.from_path(WB_PATH)
    wb.evaluate_all()
    return wb


def apply_override(wb):
    """init_revenue=800 (was 1000), margin_base=0.25 (was 0.30)."""
    wb.set_value("Inputs", 10, 3, 800)
    wb.set_value("Inputs", 17, 3, 0.25)


EXPECTED = 2311.7  # 800*0.25*(1+1.1+1.21) + 800*0.35*(1+1.2+1.44) + 800*0.25*(1+1.05+1.1025)

# Pattern A: set_value → evaluate_all
wb = reset_and_baseline()
apply_override(wb)
wb.evaluate_all()
a = wb.get_value(*TARGET)
print(f"[A] just evaluate_all:            {a}  expected {EXPECTED}  {'OK' if abs(a - EXPECTED) < 0.1 else 'WRONG'}")

# Pattern B: set_value → per-cell evaluate_cell loop in cell-address order →
# evaluate_all (mimics supermod's populate_cache + final evaluate_all)
wb = reset_and_baseline()
apply_override(wb)
for r in range(1, 30):
    for c in range(1, 8):
        try:
            if wb.get_formula("Model", r, c):
                wb.evaluate_cell("Model", r, c)
        except Exception:
            pass
wb.evaluate_all()
b = wb.get_value(*TARGET)
print(f"[B] per-cell + evaluate_all:      {b}  expected {EXPECTED}  {'OK' if abs(b - EXPECTED) < 0.1 else 'WRONG'}")

# Pattern C: same as B but in reverse cell-address order
wb = reset_and_baseline()
apply_override(wb)
for r in range(29, 0, -1):
    for c in range(7, 0, -1):
        try:
            if wb.get_formula("Model", r, c):
                wb.evaluate_cell("Model", r, c)
        except Exception:
            pass
wb.evaluate_all()
c = wb.get_value(*TARGET)
print(f"[C] reverse per-cell + evaluate_all: {c}  expected {EXPECTED}  {'OK' if abs(c - EXPECTED) < 0.1 else 'WRONG'}")
