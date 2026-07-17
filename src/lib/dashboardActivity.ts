import type { DashboardActivityBucket, WatchActivity } from "$lib/ipc/types";

const DAY_IN_MILLISECONDS = 86_400_000;

export interface DashboardActivityDay {
  date: string;
  recordedEpisodes: number;
  recordedMinutes: number;
  unknownDurationEpisodes: number;
  inRange: boolean;
}

export interface DashboardActivityWeek {
  index: number;
  label: string;
  startDate: string;
  endDate: string;
  recordedEpisodes: number;
  recordedMinutes: number;
  unknownDurationEpisodes: number;
  days: DashboardActivityDay[];
}

export interface DashboardActivityMonthMarker {
  period: string;
  label: string;
  weekIndex: number;
}

export interface DashboardActivityYear {
  rangeStart: string;
  rangeEnd: string;
  months: DashboardActivityMonthMarker[];
  weeks: DashboardActivityWeek[];
}

interface MutableActivityTotals {
  recordedEpisodes: number;
  recordedMinutes: number;
  unknownDurationEpisodes: number;
}

export function buildDashboardActivityYear(
  monthly: DashboardActivityBucket[],
  events: WatchActivity[],
): DashboardActivityYear | null {
  const periods = monthly
    .map((bucket) => parseMonthPeriod(bucket.period))
    .filter((period): period is Date => period !== null)
    .sort((left, right) => left.getTime() - right.getTime());

  if (periods.length !== 12) {
    return null;
  }

  const rangeStart = periods[0];
  const finalMonth = periods.at(-1);
  if (rangeStart === undefined || finalMonth === undefined) {
    return null;
  }
  const rangeEnd = new Date(
    Date.UTC(finalMonth.getUTCFullYear(), finalMonth.getUTCMonth() + 1, 0),
  );
  const gridStart = addUtcDays(rangeStart, -mondayIndex(rangeStart));
  const gridEnd = addUtcDays(rangeEnd, 6 - mondayIndex(rangeEnd));
  const eventTotals = aggregateEventsByUtcDay(events);
  const weekCount =
    Math.floor(
      (gridEnd.getTime() - gridStart.getTime()) / DAY_IN_MILLISECONDS / 7,
    ) + 1;

  const weeks = Array.from({ length: weekCount }, (_, weekIndex) => {
    const start = addUtcDays(gridStart, weekIndex * 7);
    const days = Array.from({ length: 7 }, (_, weekdayIndex) => {
      const date = addUtcDays(start, weekdayIndex);
      const dateKey = formatUtcDate(date);
      const totals = eventTotals.get(dateKey);
      return {
        date: dateKey,
        recordedEpisodes: totals?.recordedEpisodes ?? 0,
        recordedMinutes: totals?.recordedMinutes ?? 0,
        unknownDurationEpisodes: totals?.unknownDurationEpisodes ?? 0,
        inRange: date >= rangeStart && date <= rangeEnd,
      } satisfies DashboardActivityDay;
    });
    const totals = days.reduce<MutableActivityTotals>((summary, day) => {
      if (!day.inRange) return summary;
      summary.recordedEpisodes += day.recordedEpisodes;
      summary.recordedMinutes += day.recordedMinutes;
      summary.unknownDurationEpisodes += day.unknownDurationEpisodes;
      return summary;
    }, emptyTotals());
    return {
      index: weekIndex,
      label: `W${(weekIndex + 1).toString().padStart(2, "0")}`,
      startDate: formatUtcDate(start),
      endDate: formatUtcDate(addUtcDays(start, 6)),
      ...totals,
      days,
    } satisfies DashboardActivityWeek;
  });

  return {
    rangeStart: formatUtcDate(rangeStart),
    rangeEnd: formatUtcDate(rangeEnd),
    months: periods.map((month) => ({
      period: formatUtcMonth(month),
      label: new Intl.DateTimeFormat(undefined, {
        month: "short",
        timeZone: "UTC",
      }).format(month),
      weekIndex: Math.floor(
        (month.getTime() - gridStart.getTime()) / DAY_IN_MILLISECONDS / 7,
      ),
    })),
    weeks,
  };
}

export function contributionLevel(minutes: number): number {
  if (minutes <= 0) return 0;
  if (minutes <= 24) return 1;
  if (minutes <= 48) return 2;
  if (minutes <= 72) return 3;
  return 4;
}

function aggregateEventsByUtcDay(
  events: WatchActivity[],
): Map<string, MutableActivityTotals> {
  const totalsByDay = new Map<string, MutableActivityTotals>();
  for (const event of events) {
    const occurredAt = new Date(event.occurredAt);
    if (Number.isNaN(occurredAt.getTime())) continue;
    const date = formatUtcDate(occurredAt);
    const totals = totalsByDay.get(date) ?? emptyTotals();
    totals.recordedEpisodes += event.progressDelta;
    if (event.episodeDurationMinutes === null) {
      totals.unknownDurationEpisodes += event.progressDelta;
    } else {
      totals.recordedMinutes +=
        event.progressDelta * event.episodeDurationMinutes;
    }
    totalsByDay.set(date, totals);
  }
  return totalsByDay;
}

function parseMonthPeriod(period: string): Date | null {
  const match = /^(\d{4})-(\d{2})$/.exec(period);
  if (match === null) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  if (!Number.isInteger(year) || month < 1 || month > 12) return null;
  return new Date(Date.UTC(year, month - 1, 1));
}

function emptyTotals(): MutableActivityTotals {
  return {
    recordedEpisodes: 0,
    recordedMinutes: 0,
    unknownDurationEpisodes: 0,
  };
}

function mondayIndex(date: Date): number {
  return (date.getUTCDay() + 6) % 7;
}

function addUtcDays(date: Date, days: number): Date {
  return new Date(date.getTime() + days * DAY_IN_MILLISECONDS);
}

function formatUtcDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

function formatUtcMonth(date: Date): string {
  return date.toISOString().slice(0, 7);
}
