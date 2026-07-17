import { describe, expect, it } from "vitest";

import type { DashboardActivityBucket, WatchActivity } from "$lib/ipc/types";

import {
  buildDashboardActivityYear,
  contributionLevel,
} from "./dashboardActivity";

const monthly: DashboardActivityBucket[] = [
  "2025-08",
  "2025-09",
  "2025-10",
  "2025-11",
  "2025-12",
  "2026-01",
  "2026-02",
  "2026-03",
  "2026-04",
  "2026-05",
  "2026-06",
  "2026-07",
].map((period) => ({
  period,
  recordedEpisodes: 0,
  recordedMinutes: 0,
  unknownDurationEpisodes: 0,
}));

const events: WatchActivity[] = [
  {
    anilistId: 1,
    mediaKind: "ANIME",
    progressDelta: 2,
    episodeDurationMinutes: 24,
    occurredAt: "2026-07-16T09:00:00.000Z",
  },
  {
    anilistId: 2,
    mediaKind: "ANIME",
    progressDelta: 1,
    episodeDurationMinutes: null,
    occurredAt: "2026-07-16T10:00:00.000Z",
  },
  {
    anilistId: 3,
    mediaKind: "ANIME",
    progressDelta: 4,
    episodeDurationMinutes: 24,
    occurredAt: "2025-07-31T23:59:59.000Z",
  },
];

describe("buildDashboardActivityYear", () => {
  it("builds twelve month markers and week columns from daily activity", () => {
    const year = buildDashboardActivityYear(monthly, events);

    expect(year).not.toBeNull();
    expect(year?.rangeStart).toBe("2025-08-01");
    expect(year?.rangeEnd).toBe("2026-07-31");
    expect(year?.months).toHaveLength(12);
    expect(year?.weeks.length).toBeGreaterThanOrEqual(52);

    const activityDay = year?.weeks
      .flatMap((week) => week.days)
      .find((day) => day.date === "2026-07-16");
    expect(activityDay).toMatchObject({
      recordedEpisodes: 3,
      recordedMinutes: 48,
      unknownDurationEpisodes: 1,
      inRange: true,
    });

    const excludedDay = year?.weeks
      .flatMap((week) => week.days)
      .find((day) => day.date === "2025-07-31");
    expect(excludedDay).toMatchObject({
      recordedEpisodes: 4,
      recordedMinutes: 96,
      inRange: false,
    });
    const activityWeek = year?.weeks.find((week) =>
      week.days.some((day) => day.date === "2026-07-16"),
    );
    expect(activityWeek).toMatchObject({
      recordedEpisodes: 3,
      recordedMinutes: 48,
      unknownDurationEpisodes: 1,
    });
  });

  it("requires the authoritative twelve monthly buckets", () => {
    expect(buildDashboardActivityYear(monthly.slice(1), events)).toBeNull();
  });

  it("uses stable contribution thresholds", () => {
    expect([0, 1, 24, 25, 48, 49, 72, 73].map(contributionLevel)).toEqual([
      0, 1, 1, 2, 2, 3, 3, 4,
    ]);
  });
});
