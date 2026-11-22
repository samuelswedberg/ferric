import assert from "node:assert/strict";
import test from "node:test";
import { firstStepFor, hasPortConflict, mockOnboardingStep } from "./logic.ts";

test("new users start at why while returning signed-out users start at sign in", () => {
  assert.equal(firstStepFor({ status: "noClientId" }), 1);
  assert.equal(firstStepFor({ status: "signedOut" }), 4);
});

test("port conflict help recognizes the authenticator error", () => {
  assert.equal(hasPortConflict("port 8898 is already in use"), true);
  assert.equal(hasPortConflict("Spotify sign-in timed out"), false);
});

test("every onboarding panel step has a stable mock URL", () => {
  assert.deepEqual(
    ["onboardingWhy", "onboardingCreate", "onboardingClientId", "onboardingSignIn"].map(
      mockOnboardingStep,
    ),
    [1, 2, 3, 4],
  );
});
