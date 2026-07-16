import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { setTestViewport, testViewports } from "../../../test/viewports";

import HomePage from "./HomePage.svelte";

describe("HomePage", () => {
  it.each(testViewports)(
    "renders one truthful carousel slide at the $name viewport",
    (viewport) => {
      setTestViewport(viewport);
      render(HomePage);

      expect(
        screen.getByRole("heading", { level: 1, name: "Home" }),
      ).toBeInTheDocument();
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(screen.getByText("sample library")).toBeInTheDocument();
      expect(
        screen.getByText(/sample entries and progress are not saved/i),
      ).toBeInTheDocument();
      expect(screen.getByText(/known episodes waiting/i)).toBeInTheDocument();
      expect(screen.getByText(/schedule unknown/i)).toBeInTheDocument();

      const carousel = screen.getByRole("region", { name: "Airing anime" });
      expect(carousel).toHaveAttribute("aria-roledescription", "carousel");
      expect(
        within(carousel).getAllByRole("group", { name: /of 3:/i }),
      ).toHaveLength(1);
      expect(
        within(carousel).getByRole("group", {
          name: /1 of 3: Moonlit Post Office/i,
        }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("img", {
          name: "Sample cover artwork for Moonlit Post Office",
        }),
      ).toBeInTheDocument();
      expect(screen.getByText("on time")).toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: "all caught up" }),
      ).toBeDisabled();
      expect(
        screen.queryByRole("button", { name: /mark ep/i }),
      ).not.toBeInTheDocument();
    },
  );

  it("moves the only active carousel slide with controls and keyboard", async () => {
    const user = userEvent.setup();
    render(HomePage);

    const nextButton = screen.getByRole("button", { name: "Next anime" });
    expect(
      screen.getByRole("group", {
        name: /1 of 3: Moonlit Post Office/i,
      }),
    ).toBeInTheDocument();

    await user.click(nextButton);
    expect(
      screen.getByRole("group", {
        name: /2 of 3: Last Train to Aomori/i,
      }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("group", { name: /1 of 3:/i }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText("Slide 2 of 3, Last Train to Aomori"),
    ).toBeInTheDocument();

    nextButton.focus();
    await user.keyboard("{ArrowRight}");
    expect(
      screen.getByRole("group", {
        name: /3 of 3: Signal Lost at 2AM/i,
      }),
    ).toBeInTheDocument();

    await user.keyboard("{Home}");
    expect(
      screen.getByRole("group", {
        name: /1 of 3: Moonlit Post Office/i,
      }),
    ).toBeInTheDocument();

    await user.keyboard("{ArrowLeft}");
    expect(
      screen.getByRole("group", {
        name: /3 of 3: Signal Lost at 2AM/i,
      }),
    ).toBeInTheDocument();
  });

  it("updates sample catch-up progress and stops at the next unaired episode", async () => {
    const user = userEvent.setup();
    render(HomePage);

    await user.click(
      screen.getByRole("button", { name: "Show Last Train to Aomori" }),
    );
    const catchUpSlide = screen.getByRole("group", {
      name: /2 of 3: Last Train to Aomori/i,
    });

    expect(
      within(catchUpSlide).getByText(/2\s+episodes\s+waiting/i),
    ).toBeInTheDocument();
    const firstProgressAction = within(catchUpSlide).getByRole("button", {
      name: "preview · mark ep. 06 watched",
    });
    const actionBoundary = firstProgressAction.closest(".home-progress-action");
    expect(actionBoundary).toHaveClass("shrink-0", "sm:pt-5");
    expect(
      catchUpSlide.querySelector(".home-progress-track")?.nextElementSibling,
    ).toBe(actionBoundary);
    await user.click(firstProgressAction);
    expect(within(catchUpSlide).getByText("06 / 12")).toBeInTheDocument();
    expect(
      within(catchUpSlide).getByText(/1\s+episode\s+waiting/i),
    ).toBeInTheDocument();

    await user.click(
      within(catchUpSlide).getByRole("button", {
        name: "preview · mark ep. 07 watched",
      }),
    );
    expect(within(catchUpSlide).getByText("07 / 12")).toBeInTheDocument();
    expect(within(catchUpSlide).getByText("on time")).toBeInTheDocument();
    expect(
      within(catchUpSlide).getByRole("button", { name: "all caught up" }),
    ).toBeDisabled();
    expect(screen.getByText(/^0$/)).toBeInTheDocument();
  });

  it("keeps unknown schedules explicit and disables their quick action", async () => {
    const user = userEvent.setup();
    render(HomePage);

    await user.click(
      screen.getByRole("button", { name: "Show Signal Lost at 2AM" }),
    );
    const unknownSlide = screen.getByRole("group", {
      name: /3 of 3: Signal Lost at 2AM/i,
    });

    expect(
      within(unknownSlide).getByText("schedule unknown"),
    ).toBeInTheDocument();
    expect(
      within(unknownSlide).getByText(/no countdown or Home quick update/i),
    ).toBeInTheDocument();
    expect(
      within(unknownSlide).getByRole("button", {
        name: "schedule unavailable",
      }),
    ).toBeDisabled();
    expect(within(unknownSlide).getByText("—")).toBeInTheDocument();
  });

  it("exposes the no-active-anime preview and its Discover action", async () => {
    const user = userEvent.setup();
    const openDiscovery = vi.fn();
    render(HomePage, { openDiscovery });

    await user.click(screen.getByRole("button", { name: "empty state" }));

    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "Nothing is airing in your watching list.",
      }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("region", { name: "Airing anime" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText("／ᐠ｡ꞈ｡ᐟ＼").closest('[aria-hidden="true"]'),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /visit Discover/i }));
    expect(openDiscovery).toHaveBeenCalledOnce();
  });
});
