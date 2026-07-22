import fs from "node:fs";

const result = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const allowed = new Set([
    "document:README.md",
    "document:PROJECT.md",
    "document:PLAN.md",
    "file:src/routes/+page.svelte",
    "file:src/lib/components/AppShell.svelte",
    "file:src/lib/ipc/client.ts",
    "file:src/lib/ipc/types.ts",
    "file:src-tauri/src/main.rs",
    "file:src-tauri/src/lib.rs",
    "file:src-tauri/src/commands/mod.rs",
    "file:src-tauri/src/commands/library.rs",
    "file:src-tauri/src/commands/discovery.rs",
    "file:src-tauri/src/domain/library.rs",
    "file:src-tauri/src/domain/error.rs",
    "file:src-tauri/src/database/mod.rs",
    "file:src-tauri/src/database/library.rs",
    "file:src-tauri/src/database/cache.rs",
    "file:src-tauri/src/anilist/service.rs",
    "file:src-tauri/src/anilist/client.rs",
    "file:src-tauri/src/backups/import.rs",
    "file:src-tauri/src/recommendations/mod.rs",
    "file:src-tauri/tests/persistence.rs",
    "file:src-tauri/tests/p7_release.rs",
    "config:package.json",
    "config:src-tauri/tauri.conf.json",
    "config:src-tauri/Cargo.toml",
]);
for (const item of result.nonCodeFiles.data) {
    if (item.id.includes("0001_initial.sql")) allowed.add(item.id);
}

console.log(JSON.stringify({
    entryPointCandidates: result.entryPointCandidates,
    fanInRanking: result.fanInRanking,
    fanOutRanking: result.fanOutRanking,
    bfsTraversal: result.bfsTraversal,
    layers: result.layers,
    nonCodeCounts: Object.fromEntries(
        Object.entries(result.nonCodeFiles).map(([key, value]) => [key, value.length]),
    ),
    selectedNonCode: {
        documentation: result.nonCodeFiles.documentation,
        infrastructure: result.nonCodeFiles.infrastructure,
        data: result.nonCodeFiles.data.filter((item) => item.id.includes("migrations")),
        config: result.nonCodeFiles.config.filter((item) => allowed.has(item.id)),
    },
    clusters: result.clusters,
    selectedSummaries: Object.fromEntries(
        Object.entries(result.nodeSummaryIndex).filter(([id]) => allowed.has(id)),
    ),
}, null, 2));
