export interface TestViewport {
    name: "desktop" | "narrow";
    width: number;
    height: number;
}

export const testViewports = [
    { name: "desktop", width: 1280, height: 800 },
    { name: "narrow", width: 390, height: 844 },
] as const satisfies readonly TestViewport[];

export function setTestViewport(viewport: TestViewport): void {
    Object.defineProperties(window, {
        innerWidth: { configurable: true, value: viewport.width },
        innerHeight: { configurable: true, value: viewport.height },
    });
    window.dispatchEvent(new Event("resize"));
}
