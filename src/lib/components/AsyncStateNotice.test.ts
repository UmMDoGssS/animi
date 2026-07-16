import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { CommandError } from "$lib/ipc/types";

import AsyncStateNotice from "./AsyncStateNotice.svelte";

const retryableError: CommandError = {
  code: "ANILIST_OFFLINE",
  message: "AniList could not be reached.",
  retryable: true,
};

const terminalError: CommandError = {
  code: "INVALID_REQUEST",
  message: "This request is not valid.",
  retryable: false,
};

describe("AsyncStateNotice", () => {
  it.each([
    {
      status: "loading" as const,
      title: "Loading this view",
      description: /gathering the information/i,
      role: "status" as const,
    },
    {
      status: "empty" as const,
      title: "Nothing here yet",
      description: /no local information/i,
      role: "status" as const,
    },
    {
      status: "offline" as const,
      title: "Working offline",
      description: /cached information remains available/i,
      role: "status" as const,
    },
    {
      status: "stale" as const,
      title: "Showing cached data",
      description: /may be out of date/i,
      role: "status" as const,
    },
    {
      status: "error" as const,
      title: "This view could not load",
      description: /could not finish this request/i,
      role: "alert" as const,
    },
  ])(
    "renders the $status state with explicit terminal copy",
    ({ status, title, description, role }) => {
      render(AsyncStateNotice, { status });

      const notice = screen.getByRole(role);
      expect(notice).toHaveAttribute("data-status", status);
      expect(notice).toHaveTextContent(`status // ${status}`);
      expect(notice).toHaveTextContent(title);
      expect(notice).toHaveTextContent(description);
      expect(notice).toHaveAttribute(
        "aria-busy",
        status === "loading" ? "true" : "false",
      );
    },
  );

  it.each([
    {
      status: "offline" as const,
      expected: /cached information remains available/i,
    },
    {
      status: "stale" as const,
      expected: /showing cached data/i,
    },
  ])(
    "keeps cached-data language in the $status state",
    ({ status, expected }) => {
      render(AsyncStateNotice, { status, error: retryableError });

      const notice = screen.getByRole("status");
      expect(notice).toHaveTextContent(expected);
      expect(notice).toHaveTextContent("AniList could not be reached.");
    },
  );

  it("uses structured error copy and invokes a retryable action", async () => {
    const user = userEvent.setup();
    const onRetry = vi.fn();
    render(AsyncStateNotice, {
      status: "error",
      error: retryableError,
      onRetry,
      retryLabel: "Refresh now",
    });

    expect(screen.getByRole("alert")).toHaveTextContent(
      "AniList could not be reached.",
    );
    await user.click(screen.getByRole("button", { name: "Refresh now" }));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it.each([
    {
      name: "a terminal error",
      error: terminalError,
      onRetry: vi.fn(),
    },
    {
      name: "a retryable error without an action",
      error: retryableError,
      onRetry: undefined,
    },
  ])("does not offer retry for $name", ({ error, onRetry }) => {
    render(AsyncStateNotice, {
      status: "error",
      error,
      onRetry,
    });

    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("supports custom compact copy without losing the state label", () => {
    render(AsyncStateNotice, {
      status: "empty",
      title: "No recommendations yet",
      description: "Complete a few anime to shape this list.",
      compact: true,
    });

    const notice = screen.getByRole("status");
    expect(notice).toHaveClass("p-3");
    expect(notice).toHaveTextContent("status // empty");
    expect(notice).toHaveTextContent("No recommendations yet");
    expect(notice).toHaveTextContent(
      "Complete a few anime to shape this list.",
    );
  });
});
