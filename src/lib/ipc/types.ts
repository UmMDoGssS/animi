export interface SmokeResponse {
  service: "animi-core";
  version: string;
  message: string;
}

export interface CommandError {
  code: string;
  message: string;
  retryable: boolean;
}

export function isCommandError(value: unknown): value is CommandError {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.code === "string" &&
    typeof candidate.message === "string" &&
    typeof candidate.retryable === "boolean"
  );
}
