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

Running `cargo bench --bench comparison` generates a Criterion report under `target/criterion/` and a machine-readable CSV in `docs/RESULTS.md` when invoked via the helper script.

See also the charts embedded in the README and `docs/images/` for a visual summary.
