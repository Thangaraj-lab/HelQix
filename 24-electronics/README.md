# HELQIX electronics (24-electronics)

Offline circuit, harness and sensor checks. Python 3.10+ with `numpy`, `scipy`
and `pytest`; no network at run time and no keys anywhere.

## 1. Set up the environment

```text
python -m venv .venv
```

```text
.venv/Scripts/python -m pip install numpy scipy pytest        (Windows)
.venv/bin/python -m pip install numpy scipy pytest            (Linux, macOS)
```

## 2. Run a case

```text
.venv/Scripts/python -m helqix_circuit run examples/circuit/dc_divider.json --out out/divider
.venv/Scripts/python -m helqix_sensors run examples/sensors/temperature_bits.json --out out/temp
```

Exit 0 means `results.json` and CSVs were written, 2 means the case is wrong,
3 means the case was understood but no result was produced. The full contract
is in `../docs/priya/INTERFACE_ELECTRONICS.md`.

## 3. Verify everything

```text
.venv/Scripts/python verify.py
```

Green means every suite passed, every registered mutation was caught, the key
scan found nothing and the automation config is intact. Add `--nightly` to run
the 20 random-seeded property tests per package as well.

Before committing, install the key guard once (from this folder):

```text
cp scripts/precommit.py ../.git/hooks/pre-commit
chmod +x ../.git/hooks/pre-commit
```

The guard scans only the staged files, so it never blocks a commit because of
an unrelated dirty file.
