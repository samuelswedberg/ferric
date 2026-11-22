import type { OnboardingStep } from "./logic";

class OnboardingController {
  open = $state(false);
  step = $state<OnboardingStep>(1);

  show(step: OnboardingStep): void {
    this.step = step;
    this.open = true;
  }

  close(): void {
    this.open = false;
  }
}

export const onboarding = new OnboardingController();
