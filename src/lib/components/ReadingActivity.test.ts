import { render, screen } from "@testing-library/svelte";
import { describe, expect, it } from "vitest";
import ReadingActivity from "./ReadingActivity.svelte";
import type { DashboardActivitySummary } from "$lib/ipc/types";

const activity: DashboardActivitySummary = {
    eventCount: 2,
    recordedEpisodes: 0,
    knownDurationEpisodes: 0,
    recordedMinutes: 0,
    unknownDurationEpisodes: 0,
    recordedChapters: 5,
    recordedVolumes: 2,
    coveredFrom: "2026-09-01T00:00:00Z",
    coveredTo: "2026-09-22T00:00:00Z",
    events: [],
    monthly: [
        {
            period: "2026-09",
            recordedEpisodes: 0,
            recordedMinutes: 0,
            unknownDurationEpisodes: 0,
            recordedChapters: 5,
            recordedVolumes: 2,
        },
    ],
    yearly: [
        {
            period: "2026",
            recordedEpisodes: 0,
            recordedMinutes: 0,
            unknownDurationEpisodes: 0,
            recordedChapters: 5,
            recordedVolumes: 2,
        },
    ],
};

describe("reading activity", () => {
    it("renders separate, accessible chapter and volume series without watch time", () => {
        const { container } = render(ReadingActivity, { activity });
        expect(
            screen.getByRole("list", { name: "Monthly recorded chapters" }),
        ).toBeInTheDocument();
        expect(
            screen.getByRole("listitem", { name: "2026-09: 5 chapters" }),
        ).toBeInTheDocument();
        expect(
            screen.getByRole("listitem", { name: "2026-09: 2 volumes" }),
        ).toBeInTheDocument();
        expect(
            screen.getByRole("list", {
                name: "Yearly recorded reading activity",
            }),
        ).toHaveTextContent("5 chapters · 2 volumes");
        expect(
            screen.getByRole("region", {
                name: "Scrollable monthly chapters chart",
            }),
        ).toHaveAttribute("tabindex", "0");
        expect(container.textContent).not.toMatch(
            /minutes|episodes|watch time/i,
        );
    });
    it("explains an empty ledger", () => {
        render(ReadingActivity, { activity: { ...activity, eventCount: 0 } });
        expect(screen.getByText(/No reading activity yet/)).toBeInTheDocument();
        expect(screen.queryByRole("list")).not.toBeInTheDocument();
    });
});
