import { describe, expect, it } from "vitest";

import { isCommandError } from "./types";

import type { LibraryEntry } from "./types";

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

describe("library IPC contracts", () => {
  it("use camelCase fields and UTC RFC 3339 timestamp strings", () => {
    const entry: LibraryEntry = {
      anilistId: 101,
      mediaKind: "ANIME",
      status: "WATCHING",
      progress: 3,
      score: 8.5,
      notes: null,
      createdAt: "2026-07-16T08:00:00.000Z",
      updatedAt: "2026-07-16T08:30:00.000Z",
    };

    expect(entry.mediaKind).toBe("ANIME");
    expect(entry.updatedAt.endsWith("Z")).toBe(true);
  });
});
