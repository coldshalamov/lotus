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

## The Byte Boundary Exception

LEB128 only wins at exact byte-aligned values: \(n = 2^{7k} - 1\) (127, 16,383, 2,097,151, 268,435,455...). At these isolated local maxima, LEB128 achieves perfect density: all 7 data bits are used and the continuation bit is 0 on the last byte. Everywhere else, LEB128 pays the 0x80 continuation tax on every byte, while Lotus J2D1 pays a one-time header tax. That trade-off means Lotus dominates between byte boundaries (e.g., 128–16,382), and the gap only narrows as k grows.

| Byte Count | Value (n=2^7k-1) | LEB128 | Lotus J2D1 | Winner |
|------------|------------------|--------|------------|--------|
| 1          | 127              | 8 bits | 12 bits    | LEB128 |
| 2          | 16,383           | 16 bits| 18 bits    | LEB128 |
| 3          | 2,097,151        | 24 bits| ~25 bits   | LEB128 |
| 4          | 268,435,455      | 32 bits| ~32 bits   | Tie    |
| 5+         | > 2^35           | 40b+   | < 40b      | Lotus  |

The loss margin shrinks as k increases: 4 bits at 127, 2 bits at 16,383, about 1 bit at 2,097,151, and approaching a tie at 4+ bytes. This is not a bug in Lotus; it is the fundamental cost of bit-alignment versus byte-alignment. For all \(n \notin \{127, 16,383, 2,097,151, \ldots\}\), Lotus achieves equal or smaller encoding size than LEB128.
