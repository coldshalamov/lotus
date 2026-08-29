#!/usr/bin/env node

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(scriptDirectory, "..");
const docs = path.join(root, "docs");

const fixtureSource = fs.readFileSync(path.join(docs, "demo-fixture.js"), "utf8");
const context = { window: {} };
vm.runInNewContext(fixtureSource, context, { filename: "docs/demo-fixture.js" });
const reference = context.window.LOTUS_REFERENCE;
assert.ok(reference, "demo-fixture.js must define window.LOTUS_REFERENCE");

const coreUrl = pathToFileURL(path.join(docs, "lotus-core.mjs")).href;
const {
  U32_MAX,
  binaryField,
  leb128Bits,
  lotusBits,
  lotusLayout,
  nonnegativeWidth,
  positiveWidth,
  validateFixture,
  valuesWithinBudget,
} = await import(`${coreUrl}?contract=${Date.now()}`);

// Canonical mapping anchors from docs/FORMAT.md.
assert.equal(nonnegativeWidth(0n), 1);
assert.equal(nonnegativeWidth(1n), 1);
assert.equal(nonnegativeWidth(2n), 2);
assert.equal(nonnegativeWidth(5n), 2);
assert.equal(nonnegativeWidth(6n), 3);
assert.equal(positiveWidth(1n), 1);
assert.equal(positiveWidth(2n), 1);
assert.equal(positiveWidth(3n), 2);
assert.equal(positiveWidth(6n), 2);
assert.equal(positiveWidth(7n), 3);

const zero = lotusLayout(0n, 1, 2);
assert.equal(zero.totalBits, 4);
assert.deepEqual(zero.widths, [1, 1, 1]);
assert.equal(zero.fields.map((field) => binaryField(field.value, field.width)).join(""), "0000");

assert.equal(lotusBits(2_147_483_647n, 1, 2), 39);
assert.equal(leb128Bits(2_147_483_647n), 40);
assert.equal(lotusBits(U32_MAX, 1, 2), 40);
assert.equal(leb128Bits(U32_MAX), 40);

const validation = validateFixture(reference);
assert.equal(validation.ok, true, validation.failures.join("\n"));
assert.ok(validation.cases >= 1_000, "fixture must retain broad Rust boundary coverage");
assert.ok(validation.exact, "fixture must retain exact complete-u32 evidence");

const { aggregate, intervals } = validation.exact;
assert.equal(aggregate.values, 1n << 32n);
assert.equal(aggregate.wins + aggregate.ties + aggregate.losses, aggregate.values);
assert.ok(aggregate.wins * 100n > aggregate.values * 90n, "canonical J1D2 must win on more than 90% of u32 values");
assert.ok(aggregate.lotusBits < aggregate.lebBits, "canonical J1D2 must use fewer aggregate u32 bits than LEB128");

const lotusAtItsFinish = valuesWithinBudget(intervals, aggregate.lotusBits, "lotus");
const lebAtLotusFinish = valuesWithinBudget(intervals, aggregate.lotusBits, "leb");
assert.equal(lotusAtItsFinish.complete, true);
assert.equal(lebAtLotusFinish.complete, false);
assert.ok(lotusAtItsFinish.values > lebAtLotusFinish.values);
assert.equal(valuesWithinBudget(intervals, aggregate.lebBits, "leb").complete, true);

const html = fs.readFileSync(path.join(docs, "index.html"), "utf8");
const css = ["demo.css", "demo-inspector.css", "demo-evidence.css", "demo-responsive.css"]
  .map((file) => fs.readFileSync(path.join(docs, file), "utf8"))
  .join("\n");
const demo = fs.readFileSync(path.join(docs, "demo.mjs"), "utf8");

for (const asset of ["./demo.css", "./demo-inspector.css", "./demo-evidence.css", "./demo-responsive.css", "./demo-fixture.js", "./demo.mjs"]) {
  assert.ok(html.includes(asset), `index.html must load ${asset}`);
}
for (const id of ["race-budget", "lotus-progress", "leb-progress", "value-input", "codec-bars", "codeword", "growth-chart"]) {
  assert.ok(html.includes(`id="${id}"`), `index.html must retain #${id}`);
}
assert.ok(demo.includes('from "./lotus-core.mjs"'), "demo.mjs must use the shared canonical core");
assert.ok(css.length > 1_000, "demo.css appears unexpectedly empty");

// Generated evidence must flow through the fixture rather than reappearing as
// hand-authored presentation constants.
const generatedNumbers = [
  reference.uniformU32LebBits,
  ...reference.uniformU32
    .filter((row) => row.label === "J1D2")
    .flatMap((row) => [row.totalBits, row.wins, row.ties, row.losses]),
].filter(Boolean);
for (const source of [html, css, demo]) {
  for (const number of generatedNumbers) {
    assert.equal(source.includes(number), false, `presentation source hard-codes generated evidence ${number}`);
  }
}
assert.equal(/94\.485113/.test(`${html}\n${css}\n${demo}`), false, "strict win rate must be derived from the fixture");

console.log(
  `demo contract verified: ${validation.cases} Rust cases, ${aggregate.values} exact u32 values, ` +
  `${aggregate.wins} strict Lotus wins`,
);
