import type { Auth } from "../../bindings/Auth";

export type OnboardingStep = 1 | 2 | 3 | 4;

export function firstStepFor(auth: Auth): OnboardingStep {
  return auth.status === "noClientId" ? 1 : 4;
}

export function hasPortConflict(message: string): boolean {
  const normalized = message.toLowerCase();
  return normalized.includes("port 8898") || normalized.includes("port in use");
}

export function mockOnboardingStep(mock: string | null): OnboardingStep | null {
  if (mock === "onboardingWhy") return 1;
  if (mock === "onboardingCreate") return 2;
  if (mock === "onboardingClientId") return 3;
  if (
    mock === "onboardingSignIn" ||
    mock === "onboardingSigningIn" ||
    mock === "onboardingError" ||
    mock === "onboardingPortInUse"
  ) {
    return 4;
  }
  return null;
}
