import type { LibraryStatus } from "$lib/ipc/types";

import {
  discoveryPreviewAnime,
  type DiscoveryPreviewAnime,
} from "./discoveryPreview";

export interface DashboardPreviewLibraryEntry {
  media: DiscoveryPreviewAnime;
  libraryStatus: LibraryStatus;
  progress: number;
  score: number | null;
}

export interface DashboardPreviewActivityBucket {
  label: string;
  shortLabel: string;
  minutes: number;
  episodes: number;
}

export interface DashboardPreviewActivityRange {
  kind: "fictional-sample";
  label: string;
  coveredRange: string;
  excludedEpisodes: number;
  buckets: readonly DashboardPreviewActivityBucket[];
}

export interface DashboardPreviewActivityDay {
  kind: "fictional-sample";
  date: string;
  label: string;
  dayOfMonth: number;
  weekIndex: number;
  weekdayIndex: number;
  minutes: number;
  episodes: number;
}

export interface DashboardLibrarySummary {
  tracked: number;
  watching: number;
  completed: number;
  planning: number;
  currentProgress: number;
}

export interface DashboardGenreSignal {
  label: string;
  titleCount: number;
}

export const dashboardPreviewLibrary = [
  {
    media: discoveryPreviewAnime[0],
    libraryStatus: "WATCHING",
    progress: 8,
    score: null,
  },
  {
    media: discoveryPreviewAnime[3],
    libraryStatus: "WATCHING",
    progress: 3,
    score: null,
  },
  {
    media: discoveryPreviewAnime[2],
    libraryStatus: "COMPLETED",
    progress: 1,
    score: 9,
  },
  {
    media: discoveryPreviewAnime[4],
    libraryStatus: "COMPLETED",
    progress: 13,
    score: 8,
  },
  {
    media: discoveryPreviewAnime[1],
    libraryStatus: "PLANNING",
    progress: 0,
    score: null,
  },
  {
    media: discoveryPreviewAnime[5],
    libraryStatus: "PLANNING",
    progress: 0,
    score: null,
  },
] as const satisfies readonly DashboardPreviewLibraryEntry[];

export const monthlyActivityPreview = {
  kind: "fictional-sample",
  label: "July 2026",
  coveredRange: "Jul 01–31, 2026",
  excludedEpisodes: 2,
  buckets: [
    {
      label: "July 1–5",
      shortLabel: "W1",
      minutes: 96,
      episodes: 4,
    },
    {
      label: "July 6–12",
      shortLabel: "W2",
      minutes: 168,
      episodes: 7,
    },
    {
      label: "July 13–19",
      shortLabel: "W3",
      minutes: 144,
      episodes: 6,
    },
    {
      label: "July 20–26",
      shortLabel: "W4",
      minutes: 120,
      episodes: 5,
    },
    {
      label: "July 27–31",
      shortLabel: "W5",
      minutes: 72,
      episodes: 3,
    },
  ],
} as const satisfies DashboardPreviewActivityRange;

const julyDailyActivity = [
  [24, 1],
  [72, 3],
  [0, 0],
  [0, 0],
  [0, 0],
  [48, 2],
  [0, 0],
  [24, 1],
  [0, 0],
  [96, 4],
  [0, 0],
  [0, 0],
  [24, 1],
  [0, 0],
  [72, 3],
  [0, 0],
  [0, 0],
  [48, 2],
  [0, 0],
  [0, 0],
  [24, 1],
  [72, 3],
  [0, 0],
  [0, 0],
  [24, 1],
  [0, 0],
  [24, 1],
  [0, 0],
  [48, 2],
  [0, 0],
  [0, 0],
] as const;

export const monthlyActivityDays = julyDailyActivity.map(
  ([minutes, episodes], index): DashboardPreviewActivityDay => {
    const dayOfMonth = index + 1;
    const calendarOffset = index + 2;

    return {
      kind: "fictional-sample",
      date: `2026-07-${dayOfMonth.toString().padStart(2, "0")}`,
      label: `July ${dayOfMonth}`,
      dayOfMonth,
      weekIndex: Math.floor(calendarOffset / 7),
      weekdayIndex: calendarOffset % 7,
      minutes,
      episodes,
    };
  },
);

export const yearlyActivityPreview = {
  kind: "fictional-sample",
  label: "August 2025 through July 2026",
  coveredRange: "Aug 2025–Jul 2026",
  excludedEpisodes: 7,
  buckets: [
    { label: "August 2025", shortLabel: "A", minutes: 120, episodes: 5 },
    {
      label: "September 2025",
      shortLabel: "S",
      minutes: 240,
      episodes: 10,
    },
    { label: "October 2025", shortLabel: "O", minutes: 96, episodes: 4 },
    { label: "November 2025", shortLabel: "N", minutes: 0, episodes: 0 },
    {
      label: "December 2025",
      shortLabel: "D",
      minutes: 312,
      episodes: 13,
    },
    { label: "January 2026", shortLabel: "J", minutes: 420, episodes: 18 },
    {
      label: "February 2026",
      shortLabel: "F",
      minutes: 288,
      episodes: 12,
    },
    { label: "March 2026", shortLabel: "M", minutes: 360, episodes: 15 },
    { label: "April 2026", shortLabel: "A", minutes: 240, episodes: 10 },
    { label: "May 2026", shortLabel: "M", minutes: 504, episodes: 21 },
    { label: "June 2026", shortLabel: "J", minutes: 396, episodes: 17 },
    { label: "July 2026", shortLabel: "J", minutes: 600, episodes: 25 },
  ],
} as const satisfies DashboardPreviewActivityRange;

export function summarizeDashboardLibrary(
  entries: readonly DashboardPreviewLibraryEntry[],
): DashboardLibrarySummary {
  return entries.reduce<DashboardLibrarySummary>(
    (summary, entry) => {
      summary.tracked += 1;
      summary.currentProgress += entry.progress;

      if (entry.libraryStatus === "WATCHING") {
        summary.watching += 1;
      } else if (entry.libraryStatus === "COMPLETED") {
        summary.completed += 1;
      } else {
        summary.planning += 1;
      }

      return summary;
    },
    {
      tracked: 0,
      watching: 0,
      completed: 0,
      planning: 0,
      currentProgress: 0,
    },
  );
}

export function libraryEntriesForStatus(
  entries: readonly DashboardPreviewLibraryEntry[],
  status: LibraryStatus,
): DashboardPreviewLibraryEntry[] {
  return entries.filter((entry) => entry.libraryStatus === status);
}

export function buildGenreSignals(
  entries: readonly DashboardPreviewLibraryEntry[],
  limit = 3,
): DashboardGenreSignal[] {
  const counts = new Map<string, number>();

  for (const entry of entries) {
    if (entry.libraryStatus === "PLANNING") {
      continue;
    }

    for (const genre of entry.media.media.genres) {
      counts.set(genre, (counts.get(genre) ?? 0) + 1);
    }
  }

  return [...counts.entries()]
    .sort(([leftLabel, leftCount], [rightLabel, rightCount]) => {
      return rightCount - leftCount || leftLabel.localeCompare(rightLabel);
    })
    .slice(0, limit)
    .map(([label, titleCount]) => ({ label, titleCount }));
}

export function buildTagSampler(
  entries: readonly DashboardPreviewLibraryEntry[],
  limit = 4,
): string[] {
  return entries
    .filter((entry) => entry.libraryStatus !== "PLANNING")
    .flatMap((entry) => entry.media.media.tags)
    .sort((left, right) => right.rank - left.rank)
    .slice(0, limit)
    .map((tag) => tag.name);
}
