export const navigationItems = [
    {
        id: "home",
        label: "Home",
        icon: "home",
    },
    {
        id: "discovery",
        label: "Discover",
        icon: "discover",
    },
    {
        id: "dashboard",
        label: "Dashboard",
        icon: "dashboard",
    },
    {
        id: "settings",
        label: "Settings",
        icon: "settings",
    },
] as const;

export type PrimaryViewId = (typeof navigationItems)[number]["id"];
export type NavigationIconId = (typeof navigationItems)[number]["icon"];
