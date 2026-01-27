# Benchmark methodology

Benchmarks are driven by Criterion and aim to mirror the workload patterns from the whitepaper:

* **Small:** integers in `[0, 255]`
* **Medium:** integers from sensor-like ranges `[0, 1_000_000]` sampled every `10_000`
* **Large32:** 32-bit style range sampled sparsely
* **Large64:** 64-bit synthetic sweep (see `scripts/reproduce_paper.sh`)

Each suite measures encode throughput and average encoded size for:

* Lotus `J=2, d=1` (default)
* Lotus `J=3, d=1`
* LEB128
* Elias Delta

Running `cargo bench --bench comparison` generates a Criterion report under `target/criterion/`. The `scripts/reproduce_paper.sh` helper refreshes `docs/RESULTS.md` with the snapshot table.

For configuration tuning across multiple `(J,d)` pairs, run `python scripts/config_sweep.py` to regenerate the modeled-size table in `docs/RESULTS.md`.

See also the charts embedded in the README and `docs/images/` for a visual summary.
