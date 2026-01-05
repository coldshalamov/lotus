# Theory: where Lotus fits

Lotus sits between dense fixed-width encodings and self-delimiting universal codes. By "unfolding" the binary tree of fixed-length strings, the codec recovers representational density that byte-quantized varints leave on the table while retaining bounded prefix decoding via a short tier chain.

Key properties:

* **Density reclaiming:** the mapping `Lotus(b) = (2^|b| - 2) + value(b)` ensures every distinct bitstring is used.
* **Configurable envelope:** `(J, d)` determine how many tiers of lengths precede the payload; modest values already exceed 64-bit ranges.
* **Predictable deformation:** the positive Lotus width function `LW(v) = bit_length(v+1) - 1` keeps headers compact and stable across realistic distributions.

The construction is closely related to the Elias family but avoids gamma-style unary prefixes and byte rounding. In practice it offers a smooth size curve across scales.
