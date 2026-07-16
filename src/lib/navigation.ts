export const navigationItems = [
  {
    id: "library",
    label: "Library",
    title: "Your library",
    description: "Watching, completed, and planned anime will live here.",
  },
  {
    id: "discover",
    label: "Discover",
    title: "Discover anime",
    description:
      "Search and trending titles will appear here when metadata support lands.",
  },
  {
    id: "recommendations",
    label: "For you",
    title: "Recommendations",
    description:
      "Private, explainable recommendations will be calculated on this device.",
  },
  {
    id: "settings",
    label: "Settings",
    title: "Settings & backup",
    description:
      "Theme preferences, backups, and recovery tools will be managed here.",
  },
] as const;

export type PrimaryViewId = (typeof navigationItems)[number]["id"];

export interface DetailsView {
  id: "details";
  mediaId: number;
}

export type AppView = PrimaryViewId | DetailsView;
