import { render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { setTestViewport, testViewports } from "../../../test/viewports";

import DashboardPage from "./DashboardPage.svelte";

describe("DashboardPage", () => {
  it.each(testViewports)(
    "renders the complete fictional local ledger at the $name viewport",
    (viewport) => {
      setTestViewport(viewport);
      render(DashboardPage);

      expect(
        screen.getByRole("heading", { level: 1, name: "Dashboard" }),
      ).toBeInTheDocument();
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(
        screen.getByRole("note", {
          name: "Dashboard sample data disclosure",
        }),
      ).toHaveTextContent(/watch-time values below are fictional/i);

      const totals = screen.getByRole("group", {
        name: "Fictional sample library totals",
      });
      expect(totals).toHaveTextContent(/tracked\s*06/i);
      expect(totals).toHaveTextContent(/watching\s*02/i);
      expect(totals).toHaveTextContent(/completed\s*02/i);
      expect(totals).toHaveTextContent(/planning\s*02/i);
      expect(screen.getByText(/25 episodes/i)).toBeInTheDocument();
      expect(
        screen.getByRole("list", { name: "Sample genre frequency" }),
      ).toBeInTheDocument();

      const tags = screen.getByRole("list", { name: "Sample media tags" });
      expect(within(tags).getByText("Space")).toBeInTheDocument();
      expect(within(tags).queryByText("Cats")).not.toBeInTheDocument();
      expect(within(tags).queryByText("Food")).not.toBeInTheDocument();
      expect(tags).not.toHaveTextContent("%");

      for (const section of ["Watching", "Completed", "Planning"]) {
        expect(
          screen.getByRole("heading", { level: 3, name: section }),
        ).toBeInTheDocument();
        expect(
          within(
            screen.getByRole("list", {
              name: `${section} sample anime`,
            }),
          ).getAllByRole("listitem"),
        ).toHaveLength(2);
      }

      expect(
        screen.getAllByRole("img", { name: /Sample cover artwork/ }),
      ).toHaveLength(6);
      expect(screen.getByText("episode total unknown")).toBeInTheDocument();
    },
  );

  it("switches between accessible monthly and yearly bar ledgers", async () => {
    const user = userEvent.setup();
    render(DashboardPage);

    const monthButton = screen.getByRole("button", { name: "month" });
    const yearButton = screen.getByRole("button", { name: "year" });
    expect(monthButton).toHaveAttribute("aria-pressed", "true");
    expect(yearButton).toHaveAttribute("aria-pressed", "false");

    const monthlyChart = screen.getByRole("list", {
      name: "Fictional sample activity for July 2026",
    });
    expect(within(monthlyChart).getAllByRole("listitem")).toHaveLength(5);
    expect(
      within(monthlyChart).getByRole("listitem", {
        name: "July 1–5: 1h 36m across 4 sample episodes.",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("group", { name: "Fictional activity summary" }),
    ).toHaveTextContent(/illustrative total\s*10h/i);

    await user.click(yearButton);

    expect(yearButton).toHaveAttribute("aria-pressed", "true");
    expect(monthButton).toHaveAttribute("aria-pressed", "false");
    const yearlyChart = screen.getByRole("list", {
      name: "Fictional sample activity for August 2025 through July 2026",
    });
    expect(within(yearlyChart).getAllByRole("listitem")).toHaveLength(12);
    expect(
      within(yearlyChart).getByRole("listitem", {
        name: "November 2025: no recorded sample watch time.",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("group", { name: "Fictional activity summary" }),
    ).toHaveTextContent(/illustrative total\s*59h 36m/i);
    expect(
      screen.getByText(/7 episodes \/\/ duration unknown/i),
    ).toBeInTheDocument();
  });

  it("keeps the populated library while showing truthful no-history copy", async () => {
    const user = userEvent.setup();
    render(DashboardPage);

    await user.click(screen.getByRole("button", { name: "no history" }));

    expect(
      screen.getByRole("heading", {
        level: 3,
        name: "There is no watch history to chart yet.",
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/will not turn existing library edit dates/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("group", { name: "Activity range" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("list", { name: /Fictional sample activity/ }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText("illustrative total")).not.toBeInTheDocument();

    expect(
      screen.getByRole("list", { name: "Watching sample anime" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("list", { name: "Completed sample anime" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("list", { name: "Planning sample anime" }),
    ).toBeInTheDocument();

    const cat = screen.getByText("(=^･ω･^=)");
    expect(cat.closest('[aria-hidden="true"]')).not.toBeNull();

    await user.click(screen.getByRole("button", { name: "sample history" }));
    expect(
      screen.getByRole("group", { name: "Activity range" }),
    ).toBeInTheDocument();
  });
});
