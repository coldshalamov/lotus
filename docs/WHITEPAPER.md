# Lotus: a parametric density-reclaiming integer codec

Lotus assigns every fixed-width bitstring to a unique integer, then describes the payload width through a bounded chain of positive width fields.

The maintained specification is `docs/FORMAT.md`. The central equations are:

\[
L_0(n)=\lfloor\log_2(n+2)\rfloor
\]

for the nonnegative payload, and

\[
L_{i+1}=\lfloor\log_2(L_i+1)\rfloor
\]

for positive width descriptors.

For configuration `(J,d)`, the codeword length is:

\[
J+\sum_{i=0}^{d}L_i
\]

The most important complete-domain result is exact, not sampled: over all `2^32` unsigned 32-bit values, J1D2 uses fewer meaningful bits than LEB128 for 4,058,104,710 values (94.485113%), ties for 33,686,546 values, and loses for 203,176,040 values.

The losses are intervals near the upper portions of LEB128 byte plateaus, not merely isolated byte-border values. The broad win comes from avoiding byte quantization.

Generated evidence:

- `docs/RESULTS.md`
- `docs/results.json`
- `docs/demo-fixture.js`

Runtime throughput is measured separately with Criterion because speed depends on hardware and toolchain.
