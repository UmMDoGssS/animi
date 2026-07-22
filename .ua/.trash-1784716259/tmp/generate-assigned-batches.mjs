import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const ua = path.join(root, ".ua");
const intermediate = path.join(ua, "intermediate");
const batchIndices = [1, 4, 10, 13, 18];
const batches = JSON.parse(fs.readFileSync(path.join(intermediate, "batches.json"), "utf8")).batches;

const explicitSummaries = {
    "src-tauri/src/backups/import.rs": "Validates versioned backup documents, resolves import conflicts, and applies complete library restores within a guarded SQLite transaction.",
    "src-tauri/src/backups/mod.rs": "Defines the backup envelope and coordinates versioned export and import operations across the database and domain layers.",
    "src-tauri/src/commands/backups.rs": "Exposes thin typed Tauri commands for exporting, previewing, and importing local backup data.",
    "src-tauri/src/commands/dashboard.rs": "Exposes the typed Tauri command that assembles the home dashboard snapshot from local repositories.",
    "src-tauri/src/commands/library.rs": "Implements typed Tauri command boundaries for listing, upserting, advancing, and removing anime library entries.",
    "src-tauri/src/commands/mod.rs": "Declares the Rust command modules registered with the Tauri application runtime.",
    "src-tauri/src/commands/settings.rs": "Exposes typed commands for reading and updating persisted application settings.",
    "src-tauri/src/commands/smoke.rs": "Provides a minimal command used to verify that the frontend-to-Rust IPC boundary is healthy.",
    "src-tauri/src/database/activity.rs": "Persists and queries watch-activity records used for progress history and dashboard calculations.",
    "src-tauri/src/database/backups.rs": "Reads and writes the durable record sets required by backup export and transactional restore workflows.",
    "src-tauri/src/database/cache.rs": "Manages AniList cache records and their freshness timestamps in SQLite.",
    "src-tauri/src/database/dashboard.rs": "Queries locally authoritative data needed to construct dashboard summaries and recent activity.",
    "src-tauri/src/database/error.rs": "Maps rusqlite failures into the application's typed database error boundary.",
    "src-tauri/src/database/library.rs": "Implements transactional SQLite repository operations for anime library entries, progress updates, and joined metadata views.",
    "src-tauri/src/database/metadata.rs": "Stores and retrieves cached AniList media metadata, genres, tags, and freshness state.",
    "src-tauri/src/database/mod.rs": "Owns database initialization, migration execution, connection configuration, and repository composition for the local SQLite authority.",
    "src-tauri/src/database/records.rs": "Defines internal database record shapes used to transfer rows between repositories and domain models.",
    "src-tauri/src/database/settings.rs": "Persists application settings and applies typed defaults when values have not yet been stored.",
    "src-tauri/src/domain/dashboard.rs": "Defines dashboard view models and aggregation rules for progress, watch time, activity, and library statistics.",
    "src-tauri/src/domain/error.rs": "Defines the structured command error contract returned across the Tauri IPC boundary.",
    "src-tauri/src/domain/library.rs": "Defines validated library requests, statuses, media snapshots, activity records, and response models shared by commands and repositories.",
    "src-tauri/src/domain/mod.rs": "Collects and re-exports the Rust domain modules used across the native application.",
    "src-tauri/src/domain/settings.rs": "Defines the typed application-settings model persisted by the native database layer.",
    "src-tauri/src/domain/smoke.rs": "Defines the typed response returned by the IPC smoke-check command.",
    ".prettierrc.json": "Configures Prettier to enforce the repository's four-space indentation and Svelte-aware formatting conventions.",
    "AGENTS.md": "Repository-wide contributor guidance covering architecture boundaries, formatting, testing, accessibility, and completion expectations.",
    "PLAN.md": "Detailed delivery roadmap and implementation-status ledger for Animi's milestones, acceptance criteria, and quality gates.",
    "PROJECT.md": "Product and architecture source of truth for the local-first anime tracker, including scope, data ownership, caching, and UI constraints.",
    "README.md": "Project overview with the current feature set, technology stack, setup commands, and development workflow.",
    "package.json": "Defines Bun-managed frontend scripts and dependencies for the Svelte 5, Vite, Vitest, ESLint, and Tauri toolchain.",
    "tsconfig.json": "Configures strict TypeScript checking for the SvelteKit frontend while extending generated framework settings.",
    "src/lib/releaseHardening.test.ts": "Verifies release-hardening invariants such as semantic color contrast and production-safe presentation behavior.",
    "src/lib/state/asyncState.ts": "Defines the reusable discriminated union used to model idle, loading, success, empty, and failure states explicitly.",
    "src/lib/state/debugPresentation.ts": "Derives user-facing debug presentation options from typed asynchronous state values.",
    "src/lib/theme/theme.test.ts": "Exercises theme token completeness, palette contrast, persistence, system-preference behavior, and DOM theme application.",
    "src/lib/theme/theme.ts": "Defines supported themes, semantic token lists, persisted preference handling, and DOM theme application.",
    "src/routes/+layout.svelte": "Establishes the root SvelteKit layout and imports global application styling.",
    "src/routes/+page.svelte": "Renders the primary Animi application shell on the root route.",
    "src/test/setup.ts": "Installs deterministic browser API shims and cleanup hooks shared by frontend tests.",
    "svelte.config.js": "Configures SvelteKit for a static single-page bundle suitable for the Tauri desktop webview.",
    "vite.config.ts": "Configures Vite and Vitest for the Svelte frontend, Tauri development host behavior, and test setup.",
};

function fixtureSummary(filePath) {
    const stem = path.basename(filePath, ".json");
    const phrases = {
        "airing-empty": "AniList fixture representing a successful airing-schedule query with no upcoming episodes.",
        "airing-success": "AniList fixture representing a successful airing-schedule response used by deterministic integration tests.",
        "details-incomplete": "AniList media-details fixture with intentionally absent optional fields for incomplete-metadata handling tests.",
        "details-known": "AniList media-details fixture containing a known complete record for mapping and cache tests.",
        "duration-invalid": "AniList fixture containing an invalid duration value to exercise defensive response validation.",
        forbidden: "Mock AniList 403 response fixture used to verify structured authorization and request failure handling.",
        "library-media-success": "AniList fixture containing media records returned during library metadata refresh tests.",
        malformed: "Malformed AniList payload fixture used to verify robust GraphQL response validation.",
        offline: "Network-offline fixture used to verify retryable errors and stale local fallback behavior.",
        "partial-errors": "AniList fixture combining usable data with GraphQL errors to exercise partial-response handling.",
        "rate-limited": "Mock AniList 429 response fixture used to verify Retry-After and rate-limit behavior.",
        "recommendation-success": "AniList fixture containing recommendation candidates and ratings for deterministic ranking tests.",
        "search-success": "AniList fixture containing a successful paginated anime search response.",
        "smoke-success": "Minimal successful AniList response fixture used by native service smoke tests.",
    };
    return phrases[stem] ?? `Deterministic AniList test fixture for the ${stem.replaceAll("-", " ")} scenario.`;
}

function fileSummary(filePath) {
    if (explicitSummaries[filePath]) return explicitSummaries[filePath];
    if (filePath.includes("src-tauri/tests/fixtures/anilist/")) return fixtureSummary(filePath);
    if (filePath.includes(".svelte-kit/output/client/_app/immutable/chunks/")) {
        return "Generated, content-hashed SvelteKit client chunk containing minified runtime or application code for the production webview bundle.";
    }
    if (filePath.includes(".svelte-kit/output/client/_app/immutable/entry/")) {
        return "Generated SvelteKit client entry artifact that bootstraps the production application bundle and its hashed dependencies.";
    }
    if (filePath.includes(".svelte-kit/output/client/_app/immutable/nodes/")) {
        return "Generated SvelteKit route-node artifact for loading and rendering a compiled layout or page in the client bundle.";
    }
    return `Implements the ${path.basename(filePath)} portion of the Animi application.`;
}

function fileTags(filePath, category) {
    if (filePath.includes(".svelte-kit/output/")) return ["generated", "sveltekit", "client-bundle"];
    if (filePath.includes("tests/fixtures/anilist/")) return ["test-fixture", "anilist-api", "error-handling"];
    if (filePath.endsWith(".test.ts")) return ["test", "frontend", filePath.includes("theme") ? "theming" : "release-hardening"];
    if (filePath.includes("/commands/")) return ["api-handler", "tauri-command", "ipc-boundary"];
    if (filePath.includes("/database/")) return ["database", "repository", "sqlite"];
    if (filePath.includes("/domain/")) return ["data-model", "validation", "type-definition"];
    if (filePath.includes("/backups/")) return ["backup", "serialization", "transaction"];
    if (filePath.includes("/theme/")) return ["theming", "accessibility", "persistence"];
    if (filePath.includes("/state/")) return ["state-management", "type-definition", "presentation"];
    if (filePath.endsWith(".svelte")) return ["component", "sveltekit", "frontend"];
    if (filePath === "vite.config.ts" || filePath === "svelte.config.js" || filePath === "package.json") return ["configuration", "build-system", "frontend"];
    if (filePath === "tsconfig.json") return ["configuration", "typescript", "strict-mode"];
    if (filePath === ".prettierrc.json") return ["configuration", "formatting", "development"];
    if (category === "docs") return ["documentation", filePath === "PLAN.md" ? "roadmap" : "architecture", "project-guidance"];
    return ["application", "animi", category === "config" ? "configuration" : "source-code"];
}

function complexity(lines, structuralCount = 0) {
    if (lines > 200 || structuralCount > 20) return "complex";
    if (lines >= 50 || structuralCount > 6) return "moderate";
    return "simple";
}

function humanize(name) {
    return String(name)
        .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
        .replaceAll("_", " ")
        .replaceAll("-", " ")
        .trim()
        .toLowerCase();
}

function symbolTags(filePath, kind, name) {
    const tags = kind === "function" ? ["function", "application-logic"] : ["data-model", "type-definition"];
    if (filePath.includes("test")) tags.push("test-helper");
    else if (filePath.includes("database")) tags.push("database");
    else if (filePath.includes("backup")) tags.push("backup");
    else if (filePath.includes("domain")) tags.push("domain-model");
    else if (filePath.includes("commands")) tags.push("tauri-command");
    else if (filePath.includes(".svelte-kit")) tags.push("generated");
    else if (String(name).toLowerCase().includes("theme")) tags.push("theming");
    else tags.push("frontend");
    return [...new Set(tags)].slice(0, 5);
}

function nodeType(file) {
    if (file.fileCategory === "config") return "config";
    if (file.fileCategory === "docs") return "document";
    if (file.fileCategory === "infra") {
        if (file.path.includes(".github/workflows") || file.path.includes(".gitlab-ci") || file.path.includes(".circleci")) return "pipeline";
        if (file.path.endsWith(".tf") || file.path.endsWith(".tfvars")) return "resource";
        return "service";
    }
    if (file.fileCategory === "data") {
        if (file.path.endsWith(".sql")) return "table";
        if (/\.(graphql|proto|prisma)$/.test(file.path)) return "schema";
        if (/openapi|swagger/i.test(file.path)) return "endpoint";
    }
    return "file";
}

function fileNodeId(file) {
    const type = nodeType(file);
    return `${type}:${file.path}`;
}

function analyzeBatch(batch) {
    const extracted = JSON.parse(fs.readFileSync(path.join(ua, "tmp", `ua-file-extract-results-${batch.batchIndex}.json`), "utf8"));
    if (!extracted.scriptCompleted) throw new Error(`Extraction did not complete for batch ${batch.batchIndex}`);
    const resultByPath = new Map(extracted.results.map((result) => [result.path, result]));
    const nodes = [];
    const edges = [];

    for (const file of batch.files) {
        const result = resultByPath.get(file.path);
        const lines = result?.nonEmptyLines ?? file.sizeLines;
        const structuralCount = (result?.functions?.length ?? 0) + (result?.classes?.length ?? 0);
        const type = nodeType(file);
        const parentId = fileNodeId(file);
        nodes.push({
            id: parentId,
            type,
            name: path.basename(file.path),
            filePath: file.path,
            summary: fileSummary(file.path),
            tags: fileTags(file.path, file.fileCategory),
            complexity: complexity(lines, structuralCount),
            ...(file.path.includes(".svelte-kit/output/") ? { languageNotes: "This is generated minified output; symbol names and line-based complexity are less descriptive than in authored source." } : {}),
        });

        if (file.fileCategory === "code" && result) {
            const exported = new Set((result.exports ?? []).map((entry) => entry.name).filter(Boolean));
            const seenFunctions = new Set();
            for (const fn of result.functions ?? []) {
                if (!fn.name || seenFunctions.has(fn.name)) continue;
                const span = fn.endLine - fn.startLine + 1;
                if (span < 10 && !exported.has(fn.name)) continue;
                seenFunctions.add(fn.name);
                const id = `function:${file.path}:${fn.name}`;
                nodes.push({
                    id,
                    type: "function",
                    name: fn.name,
                    filePath: file.path,
                    lineRange: [fn.startLine, fn.endLine],
                    summary: `Implements ${humanize(fn.name)} behavior within ${path.basename(file.path)}, supporting the file's ${humanize(fileSummary(file.path).split(".")[0])}.`,
                    tags: symbolTags(file.path, "function", fn.name),
                    complexity: complexity(span),
                });
                edges.push({ source: parentId, target: id, type: "contains", direction: "forward", weight: 1.0 });
                if (exported.has(fn.name)) edges.push({ source: parentId, target: id, type: "exports", direction: "forward", weight: 0.8 });
            }

            const seenClasses = new Set();
            for (const cls of result.classes ?? []) {
                if (!cls.name || seenClasses.has(cls.name)) continue;
                const span = cls.endLine - cls.startLine + 1;
                if (span < 20 && (cls.methods?.length ?? 0) < 2 && !exported.has(cls.name)) continue;
                seenClasses.add(cls.name);
                const id = `class:${file.path}:${cls.name}`;
                nodes.push({
                    id,
                    type: "class",
                    name: cls.name,
                    filePath: file.path,
                    lineRange: [cls.startLine, cls.endLine],
                    summary: `Defines the ${cls.name} type and its role in ${humanize(fileSummary(file.path).split(".")[0])}.`,
                    tags: symbolTags(file.path, "class", cls.name),
                    complexity: complexity(span, cls.methods?.length ?? 0),
                });
                edges.push({ source: parentId, target: id, type: "contains", direction: "forward", weight: 1.0 });
                if (exported.has(cls.name)) edges.push({ source: parentId, target: id, type: "exports", direction: "forward", weight: 0.8 });
            }
        }

        if (file.fileCategory === "code") {
            for (const importedPath of batch.batchImportData[file.path] ?? []) {
                edges.push({ source: parentId, target: `file:${importedPath}`, type: "imports", direction: "forward", weight: 0.7 });
            }
        }
    }

    if (batch.batchIndex === 10) {
        edges.push(
            { source: "config:package.json", target: "config:tsconfig.json", type: "related", direction: "forward", weight: 0.5 },
            { source: "config:.prettierrc.json", target: "config:tsconfig.json", type: "related", direction: "forward", weight: 0.5 },
            { source: "document:PROJECT.md", target: "document:PLAN.md", type: "related", direction: "forward", weight: 0.5 },
            { source: "document:README.md", target: "document:PROJECT.md", type: "related", direction: "forward", weight: 0.5 },
            { source: "document:AGENTS.md", target: "document:PROJECT.md", type: "documents", direction: "forward", weight: 0.5 },
        );
    }

    const expectedImports = batch.files
        .filter((file) => file.fileCategory === "code")
        .reduce((sum, file) => sum + (batch.batchImportData[file.path]?.length ?? 0), 0);
    const actualImports = edges.filter((edge) => edge.type === "imports").length;
    if (actualImports !== expectedImports) {
        throw new Error(`Batch ${batch.batchIndex} import edge mismatch: expected ${expectedImports}, got ${actualImports}`);
    }

    return { nodes, edges, skipped: extracted.filesSkipped ?? [] };
}

function writeParts(batch, graph) {
    const partCount = Math.ceil(Math.max(graph.nodes.length / 60, graph.edges.length / 120));
    if (partCount <= 1) {
        fs.writeFileSync(path.join(intermediate, `batch-${batch.batchIndex}.json`), `${JSON.stringify({ nodes: graph.nodes, edges: graph.edges }, null, 4)}\n`);
        return 1;
    }
    const sortedPaths = batch.files.map((file) => file.path).sort((a, b) => a.localeCompare(b));
    const chunkSize = Math.ceil(sortedPaths.length / partCount);
    for (let index = 0; index < partCount; index += 1) {
        const paths = new Set(sortedPaths.slice(index * chunkSize, (index + 1) * chunkSize));
        const partNodes = graph.nodes.filter((node) => node.filePath && paths.has(node.filePath));
        const partNodeIds = new Set(partNodes.map((node) => node.id));
        const partEdges = graph.edges.filter((edge) => partNodeIds.has(edge.source));
        fs.writeFileSync(
            path.join(intermediate, `batch-${batch.batchIndex}-part-${index + 1}.json`),
            `${JSON.stringify({ nodes: partNodes, edges: partEdges }, null, 4)}\n`,
        );
    }
    return partCount;
}

const report = [];
for (const batchIndex of batchIndices) {
    const batch = batches.find((entry) => entry.batchIndex === batchIndex);
    if (!batch) throw new Error(`Missing batch ${batchIndex}`);
    const graph = analyzeBatch(batch);
    const parts = writeParts(batch, graph);
    report.push({ batchIndex, parts, nodes: graph.nodes.length, edges: graph.edges.length, skipped: graph.skipped });
}

const allowedNodeTypes = new Set(["file", "function", "class", "config", "document", "service", "table", "endpoint", "pipeline", "schema", "resource"]);
const allowedEdgeTypes = new Set(["contains", "imports", "calls", "inherits", "implements", "exports", "depends_on", "tested_by", "configures", "documents", "deploys", "migrates", "triggers", "defines_schema", "serves", "provisions", "routes", "related"]);
for (const item of report) {
    const batch = batches.find((entry) => entry.batchIndex === item.batchIndex);
    const names = fs.readdirSync(intermediate).filter((name) => new RegExp(`^batch-${item.batchIndex}(?:-part-\\d+)?\\.json$`).test(name));
    if (names.length !== item.parts) throw new Error(`Batch ${item.batchIndex} expected ${item.parts} output files, found ${names.length}`);
    const fragments = names.map((name) => ({ name, graph: JSON.parse(fs.readFileSync(path.join(intermediate, name), "utf8")) }));
    const allNodes = fragments.flatMap(({ graph }) => graph.nodes);
    const allEdges = fragments.flatMap(({ graph }) => graph.edges);
    if (allNodes.length !== item.nodes || allEdges.length !== item.edges) throw new Error(`Batch ${item.batchIndex} total mismatch after writing`);
    const allIds = allNodes.map((node) => node.id);
    if (new Set(allIds).size !== allIds.length) throw new Error(`Batch ${item.batchIndex} contains duplicate node IDs`);
    for (const file of batch.files) {
        if (!allIds.includes(fileNodeId(file))) throw new Error(`Batch ${item.batchIndex} lacks file node for ${file.path}`);
    }
    for (const node of allNodes) {
        if (!node.id || !allowedNodeTypes.has(node.type) || !node.name || !node.summary || !Array.isArray(node.tags) || node.tags.length < 3 || node.tags.length > 5 || !["simple", "moderate", "complex"].includes(node.complexity)) {
            throw new Error(`Invalid node ${node.id ?? "<missing>"} in batch ${item.batchIndex}`);
        }
        if (!node.tags.every((tag) => /^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(tag))) throw new Error(`Invalid tags on ${node.id}`);
        if ((node.type === "function" || node.type === "class") && (!Array.isArray(node.lineRange) || node.lineRange.length !== 2)) throw new Error(`Missing line range on ${node.id}`);
    }
    const allowedFiles = new Set();
    for (const imports of Object.values(batch.batchImportData)) for (const imported of imports) allowedFiles.add(`file:${imported}`);
    for (const neighbors of Object.values(batch.neighborMap ?? {})) {
        for (const neighbor of neighbors) {
            allowedFiles.add(`file:${neighbor.path}`);
            for (const symbol of neighbor.symbols ?? []) {
                allowedFiles.add(`function:${neighbor.path}:${symbol}`);
                allowedFiles.add(`class:${neighbor.path}:${symbol}`);
            }
        }
    }
    for (const { name, graph } of fragments) {
        const localIds = new Set(graph.nodes.map((node) => node.id));
        for (const edge of graph.edges) {
            if (!edge.source || !edge.target || !allowedEdgeTypes.has(edge.type) || edge.direction !== "forward" || typeof edge.weight !== "number" || edge.source === edge.target) {
                throw new Error(`Invalid edge in ${name}: ${JSON.stringify(edge)}`);
            }
            if (!localIds.has(edge.source) && !allowedFiles.has(edge.source)) throw new Error(`Invalid edge source in ${name}: ${edge.source}`);
            if (!localIds.has(edge.target) && !allowedFiles.has(edge.target)) throw new Error(`Invalid edge target in ${name}: ${edge.target}`);
        }
    }
    const expectedImports = batch.files.filter((file) => file.fileCategory === "code").reduce((sum, file) => sum + (batch.batchImportData[file.path]?.length ?? 0), 0);
    const actualImports = allEdges.filter((edge) => edge.type === "imports").length;
    if (expectedImports !== actualImports) throw new Error(`Batch ${item.batchIndex} import count mismatch after writing`);
}
console.log(JSON.stringify(report));
