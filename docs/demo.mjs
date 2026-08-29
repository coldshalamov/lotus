import {
  U64_MAX,
  binaryField,
  eliasDeltaBits,
  eliasGammaBits,
  floorLog2,
  formatPercent,
  formatRatio,
  leb128Bits,
  lotusLayout,
  validateFixture,
  valuesWithinBudget,
} from "./lotus-core.mjs";

const reference = window.LOTUS_REFERENCE;
const verification = validateFixture(reference);
const numberFormatter = new Intl.NumberFormat("en-US");
const compactFormatter = new Intl.NumberFormat("en-US", {
  notation: "compact",
  maximumFractionDigits: 3,
});

const PROFILE_PURPOSES = {
  J1D1: "Minimum overhead through 125.",
  J2D1: "One-tier density through 2³¹−3.",
  J1D2: "Minimum meaningful bits across the full u64 domain.",
  J3D1: "One-tier full-u64 traversal with one fewer descriptor.",
};

const PROFILE_NAMES = {
  J1D1: "Tiny range",
  J2D1: "Compact 31-bit",
  J1D2: "Dense u64",
  J3D1: "Fast u64",
};

const byId = (id) => document.getElementById(id);
const formatInteger = (value) => numberFormatter.format(value);
const formatCompact = (value) => compactFormatter.format(value);

function percentAsNumber(numerator, denominator) {
  return Number((numerator * 1_000_000n) / denominator) / 10_000;
}

function setText(id, value) {
  byId(id).textContent = value;
}

function setFatal(message) {
  document.body.classList.add("is-invalid");
  const warning = byId("fatal-warning");
  warning.hidden = false;
  warning.textContent = message;
}

function bootVerification() {
  const node = byId("verification");
  if (!verification.ok || !verification.exact) {
    node.classList.add("is-error");
    node.querySelector("span:last-child").textContent = "Browser/Rust verification failed; claims and interactions are disabled.";
    setFatal(`Lotus demo verification failed: ${verification.failures.join(" · ")}`);
    return false;
  }

  node.classList.add("is-ok");
  node.querySelector("span:last-child").textContent =
    `Verified ${formatInteger(BigInt(verification.cases))} Rust boundary cases + the exact 2³² aggregate.`;
  return true;
}

function renderPrimaryEvidence() {
  const { aggregate } = verification.exact;
  const bitsSaved = aggregate.lebBits - aggregate.lotusBits;

  setText("hero-win-rate", `${formatPercent(aggregate.wins, aggregate.values)}%`);
  setText("hero-bits-saved", formatInteger(bitsSaved));
  setText("hero-average-delta", formatRatio(bitsSaved, aggregate.values));

  setText("average-lotus", formatRatio(aggregate.lotusBits, aggregate.values));
  setText("average-leb", formatRatio(aggregate.lebBits, aggregate.values));
  setText("aggregate-reduction", `${formatPercent(bitsSaved, aggregate.lebBits)}%`);

  setText("wins-count", formatInteger(aggregate.wins));
  setText("ties-count", formatInteger(aggregate.ties));
  setText("losses-count", formatInteger(aggregate.losses));
  setText("wins-percent", `${formatPercent(aggregate.wins, aggregate.values)}% of the domain`);
  setText("ties-percent", `${formatPercent(aggregate.ties, aggregate.values)}% of the domain`);
  setText("losses-percent", `${formatPercent(aggregate.losses, aggregate.values)}% of the domain`);

  byId("distribution-win").style.width = `${percentAsNumber(aggregate.wins, aggregate.values)}%`;
  byId("distribution-tie").style.width = `${percentAsNumber(aggregate.ties, aggregate.values)}%`;
  byId("distribution-loss").style.width = `${percentAsNumber(aggregate.losses, aggregate.values)}%`;
}

// ---------------------------------------------------------------------------
// Exact race
// ---------------------------------------------------------------------------

const race = {
  running: false,
  frame: null,
  startedAt: 0,
  durationMs: 9_500,
  budget: 0n,
  progress: 0,
};

function setRaceButtonState() {
  const toggle = byId("race-toggle");
  toggle.classList.toggle("is-paused", !race.running);
  toggle.setAttribute("aria-label", race.running ? "Pause race" : "Start race");
}

function raceBudgetFromSlider() {
  const totalLeb = verification.exact.aggregate.lebBits;
  return (totalLeb * BigInt(byId("race-scrubber").value)) / 10_000n;
}

function renderRace(budget) {
  const { intervals, aggregate } = verification.exact;
  const boundedBudget = budget < 0n ? 0n : budget > aggregate.lebBits ? aggregate.lebBits : budget;
  race.budget = boundedBudget;
  race.progress = Number(boundedBudget) / Number(aggregate.lebBits);

  const lotus = valuesWithinBudget(intervals, boundedBudget, "lotus");
  const leb = valuesWithinBudget(intervals, boundedBudget, "leb");
  const lotusPercent = percentAsNumber(lotus.values, aggregate.values);
  const lebPercent = percentAsNumber(leb.values, aggregate.values);
  const budgetPercent = percentAsNumber(boundedBudget, aggregate.lebBits);

  setText("race-budget", formatInteger(boundedBudget));
  setText("race-budget-progress", `${formatPercent(boundedBudget, aggregate.lebBits, 3)}%`);
  setText("lotus-values", formatInteger(lotus.values));
  setText("leb-values", formatInteger(leb.values));
  setText("lotus-percent", `${formatPercent(lotus.values, aggregate.values)}%`);
  setText("leb-percent", `${formatPercent(leb.values, aggregate.values)}%`);

  byId("lotus-progress").style.width = `${lotusPercent}%`;
  byId("leb-progress").style.width = `${lebPercent}%`;
  byId("race-scrubber").value = String(Math.round(budgetPercent * 100));

  const lead = lotus.values - leb.values;
  if (boundedBudget === 0n) {
    setText("race-status", "Both codecs are on the line. The budget begins at zero.");
  } else if (lotus.complete && !leb.complete) {
    const remaining = aggregate.values - leb.values;
    setText("race-status", `Lotus has finished. LEB128 still has ${formatInteger(remaining)} values to encode.`);
  } else if (lotus.complete && leb.complete) {
    setText("race-status", "Both codecs finished. Lotus crossed the line with the smaller packed stream.");
  } else if (lead > 0n) {
    setText("race-status", `Lotus leads by ${formatInteger(lead)} encoded values under the same budget.`);
  } else if (lead < 0n) {
    setText("race-status", `LEB128 leads early by ${formatInteger(-lead)} values; byte cliffs remain ahead.`);
  } else {
    setText("race-status", "The runners are exactly level at this storage budget.");
  }
}

function pauseRace() {
  race.running = false;
  if (race.frame !== null) cancelAnimationFrame(race.frame);
  race.frame = null;
  setRaceButtonState();
}

function raceFrame(timestamp) {
  if (!race.running) return;
  const elapsed = timestamp - race.startedAt;
  const progress = Math.min(1, elapsed / race.durationMs);
  const scaled = BigInt(Math.round(progress * 1_000_000));
  const totalLeb = verification.exact.aggregate.lebBits;
  renderRace((totalLeb * scaled) / 1_000_000n);

  if (progress >= 1) {
    pauseRace();
    return;
  }
  race.frame = requestAnimationFrame(raceFrame);
}

function startRace({ restart = false } = {}) {
  const totalLeb = verification.exact.aggregate.lebBits;
  if (restart || race.budget >= totalLeb) {
    renderRace(0n);
  }
  if (race.running) return;

  race.startedAt = performance.now() - Math.max(0, Math.min(1, race.progress)) * race.durationMs;
  race.running = true;
  setRaceButtonState();
  race.frame = requestAnimationFrame(raceFrame);
}

function initialiseRace() {
  const { aggregate } = verification.exact;
  const bitsSaved = aggregate.lebBits - aggregate.lotusBits;
  const finishPosition = percentAsNumber(aggregate.lotusBits, aggregate.lebBits);

  setText("lotus-finish-bits", formatInteger(aggregate.lotusBits));
  setText("leb-finish-bits", formatInteger(aggregate.lebBits));
  setText("race-margin", `${formatInteger(bitsSaved)} bits`);
  setText(
    "race-margin-detail",
    `${formatPercent(bitsSaved, aggregate.lebBits)}% less storage across the complete packed u32 sweep.`,
  );
  byId("lotus-finish-marker").style.setProperty("--lotus-finish-position", `${finishPosition}%`);

  byId("race-toggle").addEventListener("click", () => {
    if (race.running) pauseRace();
    else startRace();
  });
  byId("race-replay").addEventListener("click", () => startRace({ restart: true }));
  byId("race-scrubber").addEventListener("input", () => {
    pauseRace();
    renderRace(raceBudgetFromSlider());
  });

  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const staticMode = new URLSearchParams(window.location.search).get("static") === "1";
  if (reducedMotion || staticMode) {
    renderRace(aggregate.lebBits);
    pauseRace();
  } else {
    renderRace(0n);
    window.setTimeout(() => startRace({ restart: true }), 550);
  }
}

// ---------------------------------------------------------------------------
// Single-value inspector
// ---------------------------------------------------------------------------

const inspector = {
  value: 2_147_483_647n,
  profileLabel: "J1D2",
};

function selectedProfile() {
  return reference.profiles.find((profile) => profile.label === inspector.profileLabel);
}

function parseInspectorValue() {
  const raw = byId("value-input").value.trim();
  if (!/^\d+$/.test(raw)) return null;
  const value = BigInt(raw);
  return value <= U64_MAX ? value : null;
}

function updatePresetState() {
  document.querySelectorAll("[data-value]").forEach((button) => {
    button.classList.toggle("active", BigInt(button.dataset.value) === inspector.value);
  });
}

function setInspectorError(message) {
  const verdict = byId("current-verdict");
  verdict.className = "current-verdict is-error";
  verdict.querySelector("strong").textContent = "Invalid unsigned integer";
  verdict.querySelector(".verdict-detail").textContent = message;
  byId("codec-bars").replaceChildren();
  byId("codeword").replaceChildren();
  setText("codeword-total", "—");
  setText("codeword-note", "Enter a decimal integer in the u64 range.");
}

function renderCodecBars(layout) {
  const lotus = layout?.totalBits ?? null;
  const codecs = [
    { key: "lotus", name: `Lotus ${inspector.profileLabel}`, note: "meaningful bits", bits: lotus },
    { key: "leb", name: "LEB128", note: "whole bytes", bits: leb128Bits(inspector.value) },
    { key: "gamma", name: "Elias γ", note: "reference", bits: eliasGammaBits(inspector.value) },
    { key: "delta", name: "Elias δ", note: "reference", bits: eliasDeltaBits(inspector.value) },
  ];
  const valid = codecs.filter((codec) => codec.bits !== null);
  const maximum = Math.max(...valid.map((codec) => codec.bits));
  const minimum = Math.min(...valid.map((codec) => codec.bits));
  const container = byId("codec-bars");
  container.replaceChildren();

  for (const codec of codecs) {
    const row = document.createElement("div");
    row.className = `codec-bar-row is-${codec.key === "lotus" ? "lotus" : codec.key === "leb" ? "leb" : "reference"}`;
    if (codec.bits === minimum) row.classList.add("is-winner");

    const label = document.createElement("div");
    label.className = "codec-bar-label";
    label.textContent = codec.name;
    const small = document.createElement("small");
    small.textContent = codec.note;
    label.appendChild(small);

    const track = document.createElement("div");
    track.className = "codec-bar-track";
    const fill = document.createElement("div");
    fill.className = "codec-bar-fill";
    fill.style.width = codec.bits === null ? "0" : `${(codec.bits / maximum) * 100}%`;
    track.appendChild(fill);

    const bits = document.createElement("div");
    bits.className = "codec-bar-bits";
    bits.textContent = codec.bits === null ? "N/A" : `${codec.bits}b`;

    row.append(label, track, bits);
    container.appendChild(row);
  }
}

function renderCodeword(layout, profile) {
  const container = byId("codeword");
  container.replaceChildren();
  setText("codeword-title", `${profile.label} field anatomy`);

  if (!layout) {
    setText("codeword-total", "out of range");
    setText("codeword-note", `The selected profile ends at ${formatInteger(BigInt(profile.max))}. Choose a wider profile.`);
    return;
  }

  for (const field of layout.fields) {
    const node = document.createElement("div");
    node.className = `codeword-field codeword-field-${field.kind}`;
    node.style.flex = `${Math.max(1, field.width)} 1 ${Math.max(52, field.width * 10)}px`;
    node.title = `${field.label}: ${field.width} meaningful bit${field.width === 1 ? "" : "s"}`;

    const head = document.createElement("div");
    head.className = "codeword-field-head";
    const label = document.createElement("span");
    label.textContent = field.label;
    const width = document.createElement("span");
    width.textContent = `${field.width}b`;
    head.append(label, width);

    const payload = document.createElement("div");
    payload.className = "codeword-field-bits";
    payload.textContent = binaryField(field.value, field.width);

    node.append(head, payload);
    container.appendChild(node);
  }

  setText("codeword-total", `${layout.totalBits} bits`);
  setText(
    "codeword-note",
    `${layout.totalBits} meaningful bits · width chain ${layout.widths.join(" → ")} · fields are emitted MSB-first and remain packed across codeword boundaries.`,
  );
}

function renderVerdict(layout) {
  const verdict = byId("current-verdict");
  const lotus = layout?.totalBits ?? null;
  const leb = leb128Bits(inspector.value);
  verdict.className = "current-verdict";

  if (lotus === null) {
    verdict.classList.add("is-error");
    verdict.querySelector("strong").textContent = `${inspector.profileLabel} is out of range`;
    verdict.querySelector(".verdict-detail").textContent = "Choose a profile that covers this integer.";
    return;
  }

  const delta = leb - lotus;
  if (delta > 0) {
    verdict.classList.add("is-win");
    verdict.querySelector("strong").textContent = `Lotus wins by ${delta} bit${delta === 1 ? "" : "s"}`;
    verdict.querySelector(".verdict-detail").textContent = `${lotus} meaningful bits versus ${leb} bits for LEB128.`;
  } else if (delta < 0) {
    verdict.classList.add("is-loss");
    verdict.querySelector("strong").textContent = `LEB128 wins here by ${-delta} bit${delta === -1 ? "" : "s"}`;
    verdict.querySelector(".verdict-detail").textContent = `Lotus does not win every value; its exact complete-domain win rate is shown above.`;
  } else {
    verdict.querySelector("strong").textContent = "Exact tie with LEB128";
    verdict.querySelector(".verdict-detail").textContent = `Both codecs use ${lotus} meaningful bits at this integer.`;
  }
}

function renderProfileTable() {
  const tbody = byId("profile-table");
  tbody.replaceChildren();
  const leb = leb128Bits(inspector.value);

  for (const profile of reference.profiles) {
    const layout = lotusLayout(inspector.value, profile.j, profile.d);
    const bits = layout?.totalBits ?? null;
    const delta = bits === null ? null : leb - bits;
    const row = document.createElement("tr");
    if (profile.label === inspector.profileLabel) row.classList.add("is-active");

    const nameCell = document.createElement("td");
    const name = document.createElement("strong");
    name.textContent = profile.label;
    nameCell.appendChild(name);

    const configCell = document.createElement("td");
    configCell.className = "mono";
    configCell.textContent = `(${profile.j}, ${profile.d})`;

    const purposeCell = document.createElement("td");
    purposeCell.className = "table-purpose";
    purposeCell.textContent = PROFILE_PURPOSES[profile.label] ?? "Recommended profile.";

    const maximumCell = document.createElement("td");
    maximumCell.className = "mono";
    maximumCell.textContent = formatInteger(BigInt(profile.max));

    const bitsCell = document.createElement("td");
    bitsCell.className = "mono";
    bitsCell.textContent = bits === null ? "N/A" : String(bits);

    const deltaCell = document.createElement("td");
    deltaCell.className = "mono";
    if (delta === null) deltaCell.textContent = "N/A";
    else if (delta > 0) {
      deltaCell.textContent = `−${delta} bits`;
      deltaCell.classList.add("table-win");
    } else if (delta < 0) {
      deltaCell.textContent = `+${-delta} bits`;
      deltaCell.classList.add("table-loss");
    } else deltaCell.textContent = "tie";

    row.append(nameCell, configCell, purposeCell, maximumCell, bitsCell, deltaCell);
    tbody.appendChild(row);
  }
}

function renderInspector() {
  const profile = selectedProfile();
  const layout = lotusLayout(inspector.value, profile.j, profile.d);
  const magnitude = inspector.value === 0n ? 0 : floorLog2(inspector.value) + 1;

  setText("chart-current-value", formatInteger(inspector.value));
  setText("value-magnitude", inspector.value === 0n ? "zero" : `${magnitude}-bit magnitude`);
  setText("profile-purpose", PROFILE_PURPOSES[profile.label] ?? "Recommended profile.");
  byId("value-input").value = inspector.value.toString();
  updatePresetState();
  renderVerdict(layout);
  renderCodecBars(layout);
  renderCodeword(layout, profile);
  renderProfileTable();
}

function initialiseInspector() {
  const select = byId("profile-select");
  for (const profile of reference.profiles) {
    const option = document.createElement("option");
    option.value = profile.label;
    option.textContent = `${profile.label} — ${PROFILE_NAMES[profile.label] ?? "recommended"}`;
    option.selected = profile.label === inspector.profileLabel;
    select.appendChild(option);
  }

  byId("value-input").addEventListener("input", () => {
    const value = parseInspectorValue();
    if (value === null) {
      setInspectorError("Use decimal digits only, between zero and u64::MAX.");
      return;
    }
    inspector.value = value;
    renderInspector();
  });

  select.addEventListener("change", () => {
    inspector.profileLabel = select.value;
    renderInspector();
  });

  document.querySelectorAll("[data-value]").forEach((button) => {
    button.addEventListener("click", () => {
      inspector.value = BigInt(button.dataset.value);
      renderInspector();
    });
  });

  renderInspector();
}

// ---------------------------------------------------------------------------
// Explanatory magnitude chart
// ---------------------------------------------------------------------------

function svgElement(name, attributes = {}) {
  const node = document.createElementNS("http://www.w3.org/2000/svg", name);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, value);
  return node;
}

function renderGrowthChart() {
  const svg = byId("growth-chart");
  svg.replaceChildren();
  const styles = getComputedStyle(document.documentElement);
  const lotusColor = styles.getPropertyValue("--lotus-bright").trim();
  const lebColor = styles.getPropertyValue("--leb").trim();
  const lineColor = "rgba(190, 201, 232, 0.14)";
  const textColor = styles.getPropertyValue("--muted").trim();
  const pad = { left: 48, right: 18, top: 18, bottom: 42 };
  const width = 720 - pad.left - pad.right;
  const height = 300 - pad.top - pad.bottom;
  const yMax = 48;
  const x = (magnitude) => pad.left + (magnitude / 32) * width;
  const y = (bits) => pad.top + height - (bits / yMax) * height;

  for (let bits = 0; bits <= yMax; bits += 8) {
    svg.appendChild(svgElement("line", {
      x1: pad.left,
      y1: y(bits),
      x2: 720 - pad.right,
      y2: y(bits),
      stroke: lineColor,
      "stroke-width": "1",
    }));
    const label = svgElement("text", {
      x: pad.left - 10,
      y: y(bits) + 4,
      fill: textColor,
      "font-size": "10",
      "text-anchor": "end",
      "font-family": "var(--mono)",
    });
    label.textContent = String(bits);
    svg.appendChild(label);
  }

  for (const magnitude of [0, 8, 16, 24, 32]) {
    const label = svgElement("text", {
      x: x(magnitude),
      y: 284,
      fill: textColor,
      "font-size": "10",
      "text-anchor": magnitude === 0 ? "start" : magnitude === 32 ? "end" : "middle",
      "font-family": "var(--mono)",
    });
    label.textContent = magnitude === 0 ? "0" : `2^${magnitude}`;
    svg.appendChild(label);
  }

  const samples = [];
  for (let magnitude = 0; magnitude <= 32; magnitude += 1) {
    const value = magnitude === 0 ? 0n : (1n << BigInt(magnitude)) - 1n;
    samples.push({
      magnitude,
      lotus: lotusLayout(value, 1, 2).totalBits,
      leb: leb128Bits(value),
    });
  }

  const pathFor = (key) => samples
    .map((sample, index) => `${index === 0 ? "M" : "L"}${x(sample.magnitude).toFixed(2)},${y(sample[key]).toFixed(2)}`)
    .join(" ");

  svg.appendChild(svgElement("path", {
    d: pathFor("leb"),
    fill: "none",
    stroke: lebColor,
    "stroke-width": "2.25",
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
    opacity: "0.88",
  }));
  svg.appendChild(svgElement("path", {
    d: pathFor("lotus"),
    fill: "none",
    stroke: lotusColor,
    "stroke-width": "2.75",
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
  }));

  for (const magnitude of [7, 14, 21, 28, 32]) {
    const sample = samples[magnitude];
    for (const [key, color] of [["lotus", lotusColor], ["leb", lebColor]]) {
      svg.appendChild(svgElement("circle", {
        cx: x(magnitude),
        cy: y(sample[key]),
        r: key === "lotus" ? "3.5" : "3",
        fill: "#12141e",
        stroke: color,
        "stroke-width": "2",
      }));
    }
  }

  const axisLabel = svgElement("text", {
    x: 14,
    y: 146,
    fill: textColor,
    "font-size": "10",
    "text-anchor": "middle",
    transform: "rotate(-90 14 146)",
  });
  axisLabel.textContent = "meaningful bits";
  svg.appendChild(axisLabel);
}

function boot() {
  if (!bootVerification()) return;
  renderPrimaryEvidence();
  initialiseRace();
  initialiseInspector();
  renderGrowthChart();
}

boot();
