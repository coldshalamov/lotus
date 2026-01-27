#!/usr/bin/env python3
"""Generate configuration tuning tables for Lotus jumpstarter/tier settings."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable
import random


@dataclass(frozen=True)
class Workload:
    name: str
    values: Iterable[int]
    description: str


def lotus_payload_width(value: int) -> int:
    """Return the minimal Lotus payload width for a non-negative integer."""
    width = 1
    while True:
        start = (1 << width) - 2
        end = (1 << (width + 1)) - 3
        if value <= end:
            return width
        width += 1


def lw_pos(value: int) -> int:
    """Positive Lotus width helper: LW(v) = bit_length(v + 1) - 1."""
    if value < 1:
        raise ValueError("LW expects positive values")
    return (value + 1).bit_length() - 1


def lotus_model_bits(value: int, j_bits: int, tiers: int) -> int | None:
    """Return modeled bit cost or None if the value overflows jumpstarter capacity."""
    payload_width = lotus_payload_width(value)
    width_chain = []
    current = payload_width
    for _ in range(tiers):
        width = lw_pos(current)
        width_chain.append(width)
        current = width
    if not width_chain:
        return None
    if width_chain[-1] > (1 << j_bits):
        return None
    return j_bits + payload_width + sum(width_chain)


def average_bits(values: Iterable[int], j_bits: int, tiers: int) -> tuple[float, float]:
    total_bits = 0
    total = 0
    covered = 0
    for value in values:
        total += 1
        bits = lotus_model_bits(value, j_bits, tiers)
        if bits is None:
            continue
        covered += 1
        total_bits += bits
    if covered == 0:
        return float("nan"), 0.0
    return total_bits / covered, covered / total


def build_workloads(seed: int = 7) -> list[Workload]:
    random.seed(seed)
    return [
        Workload(
            name="small",
            values=range(0, 256),
            description="Exact sweep of 0..255.",
        ),
        Workload(
            name="medium",
            values=range(0, 1_000_001, 1),
            description="Exact sweep of 0..1,000,000.",
        ),
        Workload(
            name="large32",
            values=(random.getrandbits(32) for _ in range(200_000)),
            description="200k uniform samples over 32-bit space.",
        ),
        Workload(
            name="large64",
            values=(random.getrandbits(64) for _ in range(200_000)),
            description="200k uniform samples over 64-bit space.",
        ),
    ]


def main() -> None:
    configs = [
        ("J1D2", 1, 2),
        ("J2D1", 2, 1),
        ("J2D2", 2, 2),
        ("J3D1", 3, 1),
    ]
    workloads = build_workloads()

    print("# Configuration sweep (modeled)\n")
    print("Model: payload width via Lotus unfolding, tier widths via LW(v)=bit_length(v+1)-1.\n")

    print("## Workload definitions\n")
    for workload in workloads:
        print(f"- **{workload.name}**: {workload.description}")

    print("\n## Results\n")
    header = "| workload | " + " | ".join(
        f"{name} avg bits (coverage)" for name, _, _ in configs
    ) + " |"
    print(header)
    print("|" + "---|" * (len(configs) + 1))

    for workload in workloads:
        row = [workload.name]
        for _, j_bits, tiers in configs:
            avg, coverage = average_bits(workload.values, j_bits, tiers)
            if coverage == 0:
                row.append("n/a (0%)")
            else:
                row.append(f"{avg:.2f} ({coverage * 100:.1f}%)")
        print("| " + " | ".join(row) + " |")


if __name__ == "__main__":
    main()
