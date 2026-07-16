import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";

import MediaCover from "./MediaCover.svelte";

describe("MediaCover", () => {
  it("renders a real cover with meaningful alternative text", () => {
    const coverUrl = "https://example.invalid/covers/anime.jpg";
    render(MediaCover, { title: "A Quiet Anime", coverUrl });

    const cover = screen.getByRole("img", {
      name: "Cover artwork for A Quiet Anime",
    });
    expect(cover).toHaveAttribute("src", coverUrl);
    expect(cover).toHaveAttribute("alt", "Cover artwork for A Quiet Anime");
    expect(cover).toHaveAttribute("loading", "lazy");
    expect(cover).toHaveClass("block", "size-full", "object-cover");
  });

  it("renders an accessible terminal-cat fallback without a URL", () => {
    render(MediaCover, { title: "Missing Signal", coverUrl: null });

    expect(
      screen.getByRole("img", {
        name: "Cover unavailable for Missing Signal",
      }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("img", {
        name: "Cover artwork for Missing Signal",
      }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText("cover unavailable").closest('[aria-hidden="true"]'),
    ).not.toBeNull();
  });

  it("switches to the fallback when the remote image fails", async () => {
    render(MediaCover, {
      title: "Broken Transmission",
      coverUrl: "https://example.invalid/covers/broken.jpg",
    });

    const cover = screen.getByRole("img", {
      name: "Cover artwork for Broken Transmission",
    });
    await fireEvent.error(cover);

    expect(
      screen.getByRole("img", {
        name: "Cover unavailable for Broken Transmission",
      }),
    ).toBeInTheDocument();
    expect(cover).not.toBeInTheDocument();
  });

  it("keeps a fluid 2:3 frame that cannot overflow its parent", () => {
    const { container } = render(MediaCover, {
      title: "Responsive Cover",
      coverUrl: null,
    });

    expect(container.firstElementChild).toHaveClass(
      "aspect-[2/3]",
      "w-full",
      "min-w-0",
      "max-w-full",
      "overflow-hidden",
    );
  });
});
