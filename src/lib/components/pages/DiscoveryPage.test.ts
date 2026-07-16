import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { setTestViewport, testViewports } from "../../../test/viewports";

import DiscoveryPage from "./DiscoveryPage.svelte";

const originalShowModal = Object.getOwnPropertyDescriptor(
  HTMLDialogElement.prototype,
  "showModal",
);
const originalClose = Object.getOwnPropertyDescriptor(
  HTMLDialogElement.prototype,
  "close",
);

beforeAll(() => {
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value(this: HTMLDialogElement) {
      this.setAttribute("open", "");
    },
  });
  Object.defineProperty(HTMLDialogElement.prototype, "close", {
    configurable: true,
    value(this: HTMLDialogElement) {
      this.removeAttribute("open");
      this.dispatchEvent(new Event("close"));
    },
  });
});

afterAll(() => {
  if (originalShowModal) {
    Object.defineProperty(
      HTMLDialogElement.prototype,
      "showModal",
      originalShowModal,
    );
  } else {
    Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  }

  if (originalClose) {
    Object.defineProperty(HTMLDialogElement.prototype, "close", originalClose);
  } else {
    Reflect.deleteProperty(HTMLDialogElement.prototype, "close");
  }
});

describe("DiscoveryPage", () => {
  it.each(testViewports)(
    "renders the sample cover collections at the $name viewport",
    (viewport) => {
      setTestViewport(viewport);
      render(DiscoveryPage);

      expect(
        screen.getByRole("heading", { level: 1, name: "Discover" }),
      ).toBeInTheDocument();
      expect(screen.getAllByRole("heading", { level: 1 })).toHaveLength(1);
      expect(
        screen.getByRole("search", { name: "Discover sample anime" }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("searchbox", { name: "Title, genre, or tag" }),
      ).toBeInTheDocument();
      expect(
        screen.getByText(/searches bundled fictional titles/i),
      ).toBeInTheDocument();

      expect(
        screen.getByRole("list", { name: "Sample trending anime" }),
      ).toBeInTheDocument();
      expect(
        screen.getByRole("list", {
          name: "Sample recommendation layout",
        }),
      ).toBeInTheDocument();
      expect(
        screen.getAllByRole("button", { name: /Open sample details for/i }),
      ).toHaveLength(6);
      expect(
        screen.getByRole("img", {
          name: "Sample cover artwork for Paper Cranes at Dawn",
        }),
      ).toBeInTheDocument();
      expect(
        screen.getByText(/not personalized rankings/i),
      ).toBeInTheDocument();
    },
  );

  it("searches bundled titles by title, genre, or tag", async () => {
    const user = userEvent.setup();
    render(DiscoveryPage);

    const search = screen.getByRole("searchbox", {
      name: "Title, genre, or tag",
    });
    await user.type(search, "MYSTERY{Enter}");

    expect(screen.getByRole("status")).toHaveTextContent(
      "1 bundled sample match for MYSTERY.",
    );
    expect(
      screen.getByRole("list", { name: "Bundled sample search results" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: "Open sample details for The Archive Beneath Rain",
      }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", {
        name: "Open sample details for Paper Cranes at Dawn",
      }),
    ).not.toBeInTheDocument();
  });

  it("shows a truthful no-match state and clears back to browse", async () => {
    const user = userEvent.setup();
    render(DiscoveryPage);

    const search = screen.getByRole("searchbox", {
      name: "Title, genre, or tag",
    });
    await user.type(search, "mecha orchestra{Enter}");

    const emptyState = screen
      .getByRole("heading", {
        level: 2,
        name: "No bundled sample titles match “mecha orchestra”.",
      })
      .closest("section");
    expect(emptyState).not.toBeNull();
    expect(
      within(emptyState as HTMLElement).getByText(/not AniList results/i),
    ).toBeInTheDocument();
    expect(
      within(emptyState as HTMLElement).getByText("ฅ(≈චᆽච≈)ฅ"),
    ).toHaveClass("whitespace-nowrap");
    expect(
      screen.queryByRole("list", { name: "Bundled sample search results" }),
    ).not.toBeInTheDocument();

    await user.click(
      within(emptyState as HTMLElement).getByRole("button", {
        name: "clear search",
      }),
    );

    expect(
      screen.getByRole("list", { name: "Sample trending anime" }),
    ).toBeInTheDocument();
    expect(search).toHaveValue("");
    await waitFor(() => expect(search).toHaveFocus());
  });

  it("opens contract-shaped details and restores focus when closed", async () => {
    const user = userEvent.setup();
    render(DiscoveryPage);

    const opener = screen.getByRole("button", {
      name: "Open sample details for Neon Shrine Maintenance Club",
    });
    await user.click(opener);

    const dialog = screen.getByRole("dialog", {
      name: "Neon Shrine Maintenance Club",
    });
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveAttribute("open");
    expect(dialog).toHaveClass("discovery-dialog");
    expect(
      within(dialog).getByText("episode count unknown"),
    ).toBeInTheDocument();
    expect(within(dialog).getByText("not yet released")).toBeInTheDocument();
    expect(
      within(dialog).getByRole("button", {
        name: "add to planning // preview only",
      }),
    ).toBeDisabled();
    expect(document.documentElement).toHaveClass("has-modal");

    const closeButton = within(dialog).getByRole("button", {
      name: "Close details",
    });
    expect(closeButton).toHaveFocus();
    await user.click(closeButton);

    await waitFor(() => expect(dialog).not.toBeInTheDocument());
    expect(document.documentElement).not.toHaveClass("has-modal");
    expect(opener).toHaveFocus();
  });

  it("shows explicit unknown sequel dates and closes on cancel", async () => {
    const user = userEvent.setup();
    render(DiscoveryPage);

    const opener = screen.getByRole("button", {
      name: "Open sample details for The Archive Beneath Rain",
    });
    await user.click(opener);
    const dialog = screen.getByRole("dialog", {
      name: "The Archive Beneath Rain",
    });

    expect(
      within(dialog).getByText("The Index Beyond Rain"),
    ).toBeInTheDocument();
    expect(
      within(dialog).getByText("release date unknown"),
    ).toBeInTheDocument();

    fireEvent(dialog, new Event("cancel", { cancelable: true }));
    await waitFor(() => expect(dialog).not.toBeInTheDocument());
    expect(opener).toHaveFocus();
  });
});
