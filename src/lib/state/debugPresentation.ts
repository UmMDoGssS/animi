import type { AsyncPresentationStatus } from "./asyncState";

export type DebugPresentationMode = "off" | AsyncPresentationStatus;

export const debugPresentationOptions: ReadonlyArray<{
  value: DebugPresentationMode;
  label: string;
  description: string;
}> = [
  {
    value: "off",
    label: "Live app",
    description: "Use real data and network states.",
  },
  {
    value: "loading",
    label: "Loading",
    description: "Play the sleeping cat loader.",
  },
  {
    value: "error",
    label: "Error",
    description: "Show a retryable app error.",
  },
  {
    value: "offline",
    label: "Offline",
    description: "Show the disconnected state.",
  },
  {
    value: "stale",
    label: "Stale",
    description: "Show the out-of-date state.",
  },
  {
    value: "empty",
    label: "Empty",
    description: "Show the no-content state.",
  },
];
