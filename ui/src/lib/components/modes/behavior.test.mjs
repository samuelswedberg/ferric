import assert from "node:assert/strict";
import test from "node:test";
import {
  nextMode,
  queueShouldBeVisible,
  stepVolume,
  UNKNOWN_VOLUME_RESET_MS,
  VOLUME_RESET_MS,
} from "./behavior.ts";

test("twenty upward notches take volume from zero to one hundred", () => {
  let volume = 0;
  for (let notch = 0; notch < 20; notch += 1) volume = stepVolume(volume, 1);
  assert.equal(volume, 100);
});

test("volume stepping clamps at both endpoints", () => {
  assert.equal(stepVolume(0, -1), 0);
  assert.equal(stepVolume(100, 1), 100);
});

test("mode cycling follows Full, Card, Strip, Full", () => {
  assert.equal(nextMode("full"), "card");
  assert.equal(nextMode("card"), "strip");
  assert.equal(nextMode("strip"), "full");
});

test("queue visibility follows Full mode and document visibility", () => {
  assert.equal(queueShouldBeVisible("full", false), true);
  assert.equal(queueShouldBeVisible("card", false), false);
  assert.equal(queueShouldBeVisible("strip", false), false);
  assert.equal(queueShouldBeVisible("full", true), false);
});

test("volume readout timing matches the brief", () => {
  assert.equal(VOLUME_RESET_MS, 1_500);
  assert.equal(UNKNOWN_VOLUME_RESET_MS, 1_000);
});
