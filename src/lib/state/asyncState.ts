import type { CommandError } from "$lib/ipc/types";

/**
 * Shared convention for asynchronous UI work. Data-bearing stale and offline
 * states remain renderable instead of collapsing into a spinner or generic error.
 */
export type AsyncState<T> =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ready"; data: T }
  | { status: "empty" }
  | { status: "offline"; data: T | null; error: CommandError }
  | { status: "stale"; data: T; refreshError: CommandError | null }
  | { status: "error"; error: CommandError };

export type AsyncPresentationStatus = Exclude<
  AsyncState<unknown>["status"],
  "idle" | "ready"
>;
