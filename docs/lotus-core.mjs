/**
 * Canonical Lotus codec math shared by the browser demo and its CI contract.
 *
 * The Rust implementation in src/lib.rs remains the source of truth. This
 * module deliberately contains only the two normative mappings and derives all
 * presentation data from them:
 *
 *   nonnegative payload: floor(log2(n + 2))
 *   positive descriptor: floor(log2(v + 1))
 */

export const U32_VALUES = 1n << 32n;
export const U32_MAX = U32_VALUES - 1n;
export const U64_MAX = (1n << 64n) - 1n;

function requireNonnegative(value, label) {
  if (typeof value !== "bigint" || value < 0n) {
    throw new RangeError(`${label} must be a nonnegative bigint`);
  }
}

export function floorLog2(value) {
  if (typeof value !== "bigint" || value <= 0n) {
    throw new RangeError("floorLog2 requires a positive bigint");
  }

  let width = -1;
  for (let cursor = value; cursor > 0n; cursor >>= 1n) {
    width += 1;
  }
  return width;
}

export function nonnegativeWidth(value) {
  requireNonnegative(value, "value");
  return floorLog2(value + 2n);
}

export function positiveWidth(value) {
  if (typeof value !== "bigint" || value < 1n) {
    throw new RangeError("positive descriptor values begin at one");
  }
  return floorLog2(value + 1n);
}

export function binaryField(value, width) {
  if (!Number.isInteger(width) || width < 1 || width > 64) {
    throw new RangeError("field width must be an integer from 1 through 64");
  }
  if (typeof value !== "bigint" || value < 0n || value >= (1n << BigInt(width))) {
    throw new RangeError("field value does not fit the requested width");
  }
  return value.toString(2).padStart(width, "0");
}

export function lotusLayout(value, jumpstarterBits, tiers) {
  requireNonnegative(value, "value");
  if (!Number.isInteger(jumpstarterBits) || jumpstarterBits < 1 || jumpstarterBits > 8) {
    throw new RangeError("jumpstarterBits must be an integer from 1 through 8");
  }
  if (!Number.isInteger(tiers) || tiers < 1) {
    throw new RangeError("tiers must be a positive integer");
  }

  const widths = [nonnegativeWidth(value)];
  for (let index = 0; index < tiers; index += 1) {
    widths.push(positiveWidth(BigInt(widths.at(-1))));
  }

  const outerWidth = widths.at(-1);
  if (outerWidth > 2 ** jumpstarterBits) {
    return null;
  }

  const fields = [
    {
      kind: "jumpstarter",
      label: "J",
      width: jumpstarterBits,
      value: BigInt(outerWidth - 1),
    },
  ];

  for (let level = widths.length - 1; level >= 1; level -= 1) {
    const fieldWidth = widths[level];
    const describedWidth = widths[level - 1];
    const intervalStart = (1n << BigInt(fieldWidth)) - 1n;
    fields.push({
      kind: "descriptor",
      label: `T${widths.length - level}`,
      width: fieldWidth,
      value: BigInt(describedWidth) - intervalStart,
    });
  }

  const payloadWidth = widths[0];
  const payloadStart = (1n << BigInt(payloadWidth)) - 2n;
  fields.push({
    kind: "payload",
    label: "P",
    width: payloadWidth,
    value: value - payloadStart,
  });

  return {
    totalBits: jumpstarterBits + widths.reduce((sum, width) => sum + width, 0),
    widths,
    fields,
  };
}

export function lotusBits(value, jumpstarterBits, tiers) {
  return lotusLayout(value, jumpstarterBits, tiers)?.totalBits ?? null;
}

export function leb128Bits(value) {
  requireNonnegative(value, "value");
  const payloadBits = value === 0n ? 1 : floorLog2(value) + 1;
  return Math.max(1, Math.ceil(payloadBits / 7)) * 8;
}

export function eliasGammaBits(value) {
  requireNonnegative(value, "value");
  return 2 * floorLog2(value + 1n) + 1;
}

export function eliasDeltaBits(value) {
  requireNonnegative(value, "value");
  const payloadExponent = floorLog2(value + 1n);
  return 2 * floorLog2(BigInt(payloadExponent + 1)) + 1 + payloadExponent;
}

/**
 * Build exact intervals over which both Lotus and LEB128 lengths are constant.
 * This mirrors the transition-point method in src/metrics.rs and never samples.
 */
export function buildComparisonIntervals(maxValue, jumpstarterBits, tiers) {
  requireNonnegative(maxValue, "maxValue");
  const endExclusive = maxValue + 1n;
  const points = new Set([0n, endExclusive]);

  const maxExponent = Math.max(1, floorLog2(endExclusive) + 1);
  for (let exponent = 1; exponent <= maxExponent; exponent += 1) {
    const power = 1n << BigInt(exponent);
    for (const point of [power - 2n, power - 1n]) {
      if (point > 0n && point < endExclusive) {
        points.add(point);
      }
    }
    if (exponent % 7 === 0 && power > 0n && power < endExclusive) {
      points.add(power);
    }
  }

  const ordered = [...points].sort((left, right) => (left < right ? -1 : left > right ? 1 : 0));
  const intervals = [];

  for (let index = 0; index < ordered.length - 1; index += 1) {
    const start = ordered[index];
    const end = ordered[index + 1];
    const lotus = lotusBits(start, jumpstarterBits, tiers);
    if (lotus === null) {
      throw new RangeError("selected Lotus profile does not cover the requested domain");
    }
    const leb = leb128Bits(start);
    intervals.push({
      start,
      end,
      count: end - start,
      lotusBits: lotus,
      lebBits: leb,
      outcome: lotus < leb ? "win" : lotus === leb ? "tie" : "loss",
    });
  }

  return intervals;
}

export function aggregateIntervals(intervals, endInclusive = null) {
  if (!Array.isArray(intervals) || intervals.length === 0) {
    throw new TypeError("aggregateIntervals requires at least one interval");
  }

  const domainEnd = intervals.at(-1).end - 1n;
  const limit = endInclusive === null ? domainEnd : endInclusive;
  requireNonnegative(limit, "endInclusive");
  if (limit > domainEnd) {
    throw new RangeError("endInclusive exceeds the interval domain");
  }

  const result = {
    values: 0n,
    lotusBits: 0n,
    lebBits: 0n,
    wins: 0n,
    ties: 0n,
    losses: 0n,
  };
  const stop = limit + 1n;

  for (const interval of intervals) {
    if (interval.start >= stop) {
      break;
    }
    const coveredEnd = interval.end < stop ? interval.end : stop;
    const count = coveredEnd - interval.start;
    if (count <= 0n) {
      continue;
    }

    result.values += count;
    result.lotusBits += count * BigInt(interval.lotusBits);
    result.lebBits += count * BigInt(interval.lebBits);
    if (interval.outcome === "win") result.wins += count;
    if (interval.outcome === "tie") result.ties += count;
    if (interval.outcome === "loss") result.losses += count;
  }

  return result;
}

/**
 * Exact race primitive: under one shared bit budget, return how many consecutive
 * domain values a codec can encode. Every interval has a constant cost/value.
 */
export function valuesWithinBudget(intervals, budget, codec) {
  if (codec !== "lotus" && codec !== "leb") {
    throw new RangeError("codec must be 'lotus' or 'leb'");
  }
  if (typeof budget !== "bigint" || budget < 0n) {
    throw new RangeError("budget must be a nonnegative bigint");
  }

  const key = codec === "lotus" ? "lotusBits" : "lebBits";
  let values = 0n;
  let spent = 0n;

  for (const interval of intervals) {
    const bitsPerValue = BigInt(interval[key]);
    const intervalCost = interval.count * bitsPerValue;
    if (spent + intervalCost <= budget) {
      spent += intervalCost;
      values += interval.count;
      continue;
    }

    const affordable = (budget - spent) / bitsPerValue;
    spent += affordable * bitsPerValue;
    values += affordable;
    return {
      values,
      spent,
      complete: false,
      nextValue: values,
      bitsPerValue: Number(bitsPerValue),
    };
  }

  return {
    values,
    spent,
    complete: true,
    nextValue: null,
    bitsPerValue: null,
  };
}

function equalBigInt(actual, expected) {
  return actual === BigInt(expected);
}

export function validateFixture(reference) {
  const failures = [];
  const fail = (message) => {
    if (failures.length < 12) failures.push(message);
  };

  if (!reference || reference.format !== 1) {
    return { ok: false, cases: 0, failures: ["missing or unsupported Rust fixture"] };
  }
  if (!Array.isArray(reference.profiles) || !Array.isArray(reference.values) || !Array.isArray(reference.bits)) {
    return { ok: false, cases: 0, failures: ["malformed Rust fixture"] };
  }
  if (reference.values.length !== reference.bits.length) {
    fail("fixture value and bit-matrix lengths differ");
  }

  let cases = 0;
  const rows = Math.min(reference.values.length, reference.bits.length);
  for (let valueIndex = 0; valueIndex < rows; valueIndex += 1) {
    const value = BigInt(reference.values[valueIndex]);
    const expectedRow = reference.bits[valueIndex];
    if (!Array.isArray(expectedRow) || expectedRow.length !== reference.profiles.length) {
      fail(`fixture row ${valueIndex} has the wrong profile width`);
      continue;
    }

    reference.profiles.forEach((profile, profileIndex) => {
      const actual = lotusBits(value, profile.j, profile.d);
      const expected = expectedRow[profileIndex];
      if (actual !== expected) {
        fail(`${profile.label} at ${value}: JavaScript=${actual}, Rust=${expected}`);
      }
      cases += 1;
    });
  }

  for (const profile of reference.profiles) {
    const maximum = BigInt(profile.max);
    if (lotusBits(maximum, profile.j, profile.d) === null) {
      fail(`${profile.label} rejects its generated maximum`);
    }
    if (maximum < U64_MAX && lotusBits(maximum + 1n, profile.j, profile.d) !== null) {
      fail(`${profile.label} accepts a value above its generated maximum`);
    }
  }

  const exactRow = reference.uniformU32?.find((row) => row.label === "J1D2");
  let exact = null;
  if (!exactRow || exactRow.totalBits === null) {
    fail("fixture lacks exact complete-u32 J1D2 evidence");
  } else {
    const intervals = buildComparisonIntervals(U32_MAX, 1, 2);
    const aggregate = aggregateIntervals(intervals);
    exact = { intervals, aggregate, fixture: exactRow };

    const checks = [
      ["complete-u32 values", aggregate.values, reference.uniformU32Values],
      ["J1D2 total bits", aggregate.lotusBits, exactRow.totalBits],
      ["LEB128 total bits", aggregate.lebBits, reference.uniformU32LebBits],
      ["J1D2 wins", aggregate.wins, exactRow.wins],
      ["J1D2 ties", aggregate.ties, exactRow.ties],
      ["J1D2 losses", aggregate.losses, exactRow.losses],
    ];
    for (const [label, actual, expected] of checks) {
      if (!equalBigInt(actual, expected)) {
        fail(`${label}: JavaScript=${actual}, Rust=${expected}`);
      }
    }
    if (aggregate.wins + aggregate.ties + aggregate.losses !== aggregate.values) {
      fail("complete-u32 outcome counts do not partition the domain");
    }
  }

  return { ok: failures.length === 0, cases, failures, exact };
}

export function formatPercent(numerator, denominator, decimals = 6) {
  if (typeof numerator !== "bigint" || typeof denominator !== "bigint" || denominator <= 0n) {
    throw new RangeError("formatPercent requires bigint counts and a positive denominator");
  }
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 12) {
    throw new RangeError("decimals must be an integer from 0 through 12");
  }

  const scale = 10n ** BigInt(decimals);
  const scaled = (numerator * 100n * scale + denominator / 2n) / denominator;
  const whole = scaled / scale;
  if (decimals === 0) return whole.toString();
  return `${whole}.${(scaled % scale).toString().padStart(decimals, "0")}`;
}

export function formatRatio(numerator, denominator, decimals = 6) {
  if (typeof numerator !== "bigint" || typeof denominator !== "bigint" || denominator <= 0n) {
    throw new RangeError("formatRatio requires bigint counts and a positive denominator");
  }
  if (!Number.isInteger(decimals) || decimals < 0 || decimals > 12) {
    throw new RangeError("decimals must be an integer from 0 through 12");
  }

  const scale = 10n ** BigInt(decimals);
  const scaled = (numerator * scale + denominator / 2n) / denominator;
  const whole = scaled / scale;
  if (decimals === 0) return whole.toString();
  return `${whole}.${(scaled % scale).toString().padStart(decimals, "0")}`;
}
