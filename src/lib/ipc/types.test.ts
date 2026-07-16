import { describe, expect, it } from "vitest";

import { isCommandError } from "./types";

describe("isCommandError", () => {
  it("accepts only the complete structured error shape", () => {
    expect(
      isCommandError({
        code: "OFFLINE",
        message: "No connection",
        retryable: true,
      }),
    ).toBe(true);
    expect(isCommandError({ code: "OFFLINE", message: "No connection" })).toBe(
      false,
    );
    expect(isCommandError("OFFLINE")).toBe(false);
  });
});
