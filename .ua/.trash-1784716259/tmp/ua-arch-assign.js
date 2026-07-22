import fs from "node:fs";

const [inputPath, outputPath] = process.argv.slice(2);
const input = JSON.parse(fs.readFileSync(inputPath, "utf8"));

const layers = [
    {
        id: "layer:frontend-ui",
        name: "Frontend UI",
        description: "Svelte routes, pages, controls, responsive styling, and the accessible desktop presentation for Animi's library, discovery, dashboard, and settings flows.",
        nodeIds: [],
    },
    {
        id: "layer:frontend-integration",
        name: "Frontend Integration",
        description: "Typed frontend IPC contracts, client adapters, local presentation state, navigation models, and persisted theme behavior that connect the Svelte UI to native capabilities.",
        nodeIds: [],
    },
    {
        id: "layer:native-ipc-runtime",
        name: "Native IPC Runtime",
        description: "Tauri bootstrap code and thin command handlers that expose Animi's Rust-owned operations to the frontend through the typed camelCase IPC boundary.",
        nodeIds: [],
    },
    {
        id: "layer:native-services-domain",
        name: "Native Services and Domain",
        description: "Rust domain models and application services for AniList access, cache-aware discovery, recommendations, backups, validation, and business rules.",
        nodeIds: [],
    },
    {
        id: "layer:data-persistence",
        name: "Data Persistence",
        description: "SQLite repositories, migration-defined tables, cache records, and transactional storage behavior for Animi's authoritative local data.",
        nodeIds: [],
    },
    {
        id: "layer:test",
        name: "Tests and Fixtures",
        description: "Frontend component tests, Rust integration suites, and deterministic local fixtures that verify IPC, migrations, offline behavior, accessibility, and data safety.",
        nodeIds: [],
    },
    {
        id: "layer:generated-frontend",
        name: "Generated Frontend Artifacts",
        description: "Generated SvelteKit declarations, route metadata, runtime modules, and compiled client and server assets produced from Animi's frontend source.",
        nodeIds: [],
    },
    {
        id: "layer:config",
        name: "Configuration and Release Tooling",
        description: "Frontend and Rust build configuration, Tauri permissions and packaging metadata, generated manifests, repository conventions, test data manifests, and release audit tooling.",
        nodeIds: [],
    },
    {
        id: "layer:documentation",
        name: "Documentation",
        description: "Product architecture, delivery roadmap, contributor rules, release evidence, licensing notices, and fixture-maintenance guidance for Animi.",
        nodeIds: [],
    },
];

const layerById = new Map(layers.map((layer) => [layer.id, layer]));

function selectLayer(node) {
    const filePath = (node.filePath ?? "").replaceAll("\\", "/");
    const tags = new Set(node.tags ?? []);

    if (node.type === "document") return "layer:documentation";
    if (node.type === "config") return "layer:config";
    if (["table", "schema", "endpoint"].includes(node.type)) return "layer:data-persistence";
    if (filePath.startsWith(".svelte-kit/")) return "layer:generated-frontend";
    if (
        /(^|\/)(tests?|__tests__|specs?)(\/|$)/i.test(filePath) ||
        /\.(test|spec)\.[^.]+$/i.test(filePath) ||
        filePath === "src-tauri/src/anilist/tests.rs" ||
        tags.has("test") ||
        tags.has("test-fixture")
    ) return "layer:test";
    if (filePath.startsWith("src-tauri/src/database/")) return "layer:data-persistence";
    if (filePath.startsWith("src-tauri/src/commands/") || /src-tauri\/src\/(lib|main)\.rs$/i.test(filePath)) return "layer:native-ipc-runtime";
    if (/^src-tauri\/src\/(anilist|backups|domain|recommendations)\//i.test(filePath)) return "layer:native-services-domain";
    if (/^src\/lib\/(ipc|state|theme)\//i.test(filePath) || filePath === "src/lib/navigation.ts") return "layer:frontend-integration";
    if (filePath.startsWith("src/")) return "layer:frontend-ui";
    if (
        filePath.startsWith(".ua/") ||
        filePath.startsWith("scripts/") ||
        filePath.startsWith("src-tauri/capabilities/") ||
        filePath.startsWith("src-tauri/gen/") ||
        filePath.startsWith("src-tauri/icons/") ||
        filePath === "src-tauri/build.rs" ||
        filePath === ".gitattributes" ||
        filePath === ".prettierignore" ||
        /(^|\/)(eslint\.config\.js|svelte\.config\.js|vite\.config\.ts)$/i.test(filePath)
    ) return "layer:config";
    throw new Error(`No layer rule matched ${node.id} (${filePath})`);
}

for (const node of input.fileNodes) {
    layerById.get(selectLayer(node)).nodeIds.push(node.id);
}

const assigned = layers.flatMap((layer) => layer.nodeIds);
const uniqueAssigned = new Set(assigned);
if (layers.length < 3 || layers.length > 10) throw new Error(`Invalid layer count: ${layers.length}`);
if (layers.some((layer) => layer.nodeIds.length === 0)) throw new Error("At least one layer is empty");
if (assigned.length !== input.fileNodes.length || uniqueAssigned.size !== input.fileNodes.length) {
    throw new Error(`Assignment mismatch: assigned=${assigned.length}, unique=${uniqueAssigned.size}, expected=${input.fileNodes.length}`);
}

fs.writeFileSync(outputPath, `${JSON.stringify(layers, null, 4)}\n`);
for (const layer of layers) console.log(`${layer.name}: ${layer.nodeIds.length}`);
