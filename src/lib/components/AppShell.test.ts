import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import { setTestViewport, testViewports } from "../../test/viewports";

import AppShell from "./AppShell.svelte";

describe("AppShell", () => {
  it.each(testViewports)(
    "navigates between each shell view at the $name viewport",
    async (viewport) => {
      setTestViewport(viewport);
      render(AppShell);

      expect(
        screen.getByRole("heading", { level: 1, name: "Your library" }),
      ).toBeInTheDocument();

      await fireEvent.click(screen.getByRole("button", { name: "Discover" }));
      expect(
        screen.getByRole("heading", { level: 1, name: "Discover anime" }),
      ).toBeInTheDocument();

      await fireEvent.click(
        screen.getByRole("button", { name: "Preview details view" }),
      );
      expect(
        screen.getByRole("heading", { level: 1, name: "Media details" }),
      ).toBeInTheDocument();

      await fireEvent.click(
        screen.getByRole("button", { name: "Back to discover" }),
      );

      await fireEvent.click(screen.getByRole("button", { name: "For you" }));
      expect(
        screen.getByRole("heading", { level: 1, name: "Recommendations" }),
      ).toBeInTheDocument();

      await fireEvent.click(screen.getByRole("button", { name: "Settings" }));
      expect(
        screen.getByRole("heading", { level: 1, name: "Settings & backup" }),
      ).toBeInTheDocument();
    },
  );
});
