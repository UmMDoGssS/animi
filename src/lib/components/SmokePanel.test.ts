import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import type { SmokeResponse } from "$lib/ipc/types";

import SmokePanel from "./SmokePanel.svelte";

describe("SmokePanel", () => {
  it("renders a typed success response", async () => {
    const response: SmokeResponse = {
      service: "animi-core",
      version: "0.1.0",
      message: "Local core is ready",
    };
    const runCheck = vi.fn().mockResolvedValue(response);

    render(SmokePanel, { runCheck });
    await fireEvent.click(screen.getByRole("button", { name: "Check core" }));

    expect(await screen.findByRole("status")).toHaveTextContent(
      "Local core is ready · v0.1.0",
    );
  });

  it("terminates with a structured command error", async () => {
    const runCheck = vi.fn().mockRejectedValue({
      code: "SMOKE_FAILURE",
      message: "Requested smoke failure",
      retryable: false,
    });

    render(SmokePanel, { runCheck });
    await fireEvent.click(screen.getByRole("button", { name: "Check core" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Requested smoke failure (SMOKE_FAILURE)",
    );
  });
});
