import { invoke } from "@tauri-apps/api/core";

import type { CommandError, SmokeResponse } from "./types";
import { isCommandError } from "./types";

export async function checkBackend(
  simulateError = false,
): Promise<SmokeResponse> {
  return invoke<SmokeResponse>("smoke_check", { simulateError });
}

export function normalizeCommandError(error: unknown): CommandError {
  if (isCommandError(error)) {
    return error;
  }

  return {
    code: "UNEXPECTED_ERROR",
    message: "Animi could not reach its local core.",
    retryable: true,
  };
}
