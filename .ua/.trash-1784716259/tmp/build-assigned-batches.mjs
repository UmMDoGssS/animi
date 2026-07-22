import fs from "node:fs";
import path from "node:path";

const projectRoot = process.cwd();
const uaDir = path.join(projectRoot, ".ua");
const batchesDocument = JSON.parse(
    fs.readFileSync(path.join(uaDir, "intermediate", "batches.json"), "utf8"),
);
const assigned = new Set([2, 3, 5, 9, 15]);

const basename = (filePath) => filePath.replaceAll("\\", "/").split("/").at(-1);
const humanize = (value) =>
    value
        .replace(/\$/g, "")
        .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
        .replaceAll("_", " ")
        .replaceAll("-", " ")
        .trim()
        .toLowerCase();

function complexity(nonEmptyLines, structures = 0) {
    if (nonEmptyLines > 200 || structures > 30) return "complex";
    if (nonEmptyLines >= 50 || structures > 8) return "moderate";
    return "simple";
}

function fileRole(filePath) {
    const roles = {
        ".svelte-kit/output/server/chunks/exports.js":
            "Generated SvelteKit server export helpers for stores, URL validation, tracking, and response utilities used by the compiled application.",
        ".svelte-kit/output/server/chunks/false.js":
            "Generated SvelteKit stub module that exports the constant false for disabled build-time capabilities.",
        ".svelte-kit/output/server/chunks/index.js":
            "Generated Svelte server-rendering runtime providing HTML escaping, attributes, styles, context, stores, hashing, and component rendering primitives.",
        ".svelte-kit/output/server/chunks/internal.js":
            "Generated SvelteKit internal server helpers that expose framework errors, hooks, environment access, and request-local state.",
        ".svelte-kit/output/server/chunks/root.js":
            "Generated Svelte runtime chunk implementing reactive sources, derived values, effects, hydration, component mounting, and legacy component adapters.",
        ".svelte-kit/output/server/chunks/server.js":
            "Generated SvelteKit public server API surface for HTTP errors, redirects, JSON responses, and action failure values.",
        ".svelte-kit/output/server/chunks/utils.js":
            "Generated SvelteKit server utilities for form-data decoding, validation issues, cloning, content negotiation, error handling, and serialization.",
        ".svelte-kit/output/server/chunks/utils2.js":
            "Generated SvelteKit path and Base64 utility chunk shared by server-side runtime modules.",
        ".svelte-kit/output/server/entries/fallbacks/error.svelte.js":
            "Compiled SvelteKit fallback error page that renders request status and error messages when route rendering fails.",
        ".svelte-kit/output/server/entries/pages/_page.svelte.js":
            "Compiled application page bundle containing Animi navigation, loading art, async notices, the home experience, and the top-level shell.",
        ".svelte-kit/output/server/index.js":
            "Generated SvelteKit server engine that resolves routes, runs actions and loaders, manages cookies and fetches, renders responses, and handles remote calls.",
        ".svelte-kit/output/server/internal.js":
            "Generated SvelteKit internal barrel that re-exports runtime environment and request helpers for the server bundle.",
        ".svelte-kit/output/server/remote-entry.js":
            "Generated SvelteKit remote-function runtime implementing commands, forms, queries, live resources, batching, validation, and shared iterators.",
        "src/lib/components/AppShell.svelte":
            "Top-level Animi UI coordinator that switches primary views, initializes theme and local data, and wires dashboard, discovery, home, settings, backup, and library services.",
        "src/lib/components/AppShell.test.ts":
            "Component tests for application-shell navigation, theme initialization, responsive behavior, and injected service orchestration.",
        "src/lib/components/P4Flow.test.ts":
            "End-to-end-style component flow tests for discovery, media details, library mutations, dashboard updates, and persistence-facing UI behavior.",
        "src/lib/components/pages/DashboardPage.svelte":
            "Library dashboard page with insights, activity visualization, filtering and sorting, metadata refresh, progress editing, and media detail controls.",
        "src/lib/components/pages/DashboardPage.test.ts":
            "Component tests covering dashboard insights, filters, sorting, responsive layouts, metadata refresh, progress changes, and error states.",
        "src/lib/components/pages/DiscoveryPage.svelte":
            "Anime discovery page that coordinates trending titles, recommendations, debounced search, library membership, pagination, and media details.",
        "src/lib/components/pages/DiscoveryPage.test.ts":
            "Component tests for discovery loading, search races, recommendations, pagination, library mutations, details, and responsive states.",
        "src/lib/components/pages/HomePage.svelte":
            "Home page for upcoming episodes and catch-up progress, with periodic refresh, carousel navigation, library mutation, and media details.",
        "src/lib/components/pages/HomePage.test.ts":
            "Component tests for home airing order, countdown and catch-up behavior, progress updates, details, refresh, and responsive presentation.",
        "src/lib/components/pages/SettingsPage.svelte":
            "Settings page for theme selection, debug presentation modes, versioned backup export/import, and explicitly confirmed local-data deletion.",
        "src/lib/components/pages/SettingsPage.test.ts":
            "Component tests for theme controls, backup workflows, import previews, destructive-data confirmation, and settings error handling.",
        "src/test/viewports.ts":
            "Shared test utility defining representative narrow and desktop viewport dimensions and applying them to the DOM test environment.",
        "src-tauri/src/anilist/client.rs":
            "Typed AniList GraphQL client that maps network and contract errors, honors rate limits and retry windows, and converts API responses into stable domain-facing media data.",
        "src-tauri/src/anilist/mod.rs":
            "AniList module boundary that organizes the transport client, GraphQL queries, caching service, response types, and deterministic tests.",
        "src-tauri/src/anilist/queries.rs":
            "Static AniList GraphQL query documents for anime search, trending pages, library batches, media details, relations, and airing schedules.",
        "src-tauri/src/anilist/service.rs":
            "Application-facing AniList service that validates requests, applies cache policies and stale fallbacks, refreshes library metadata, and feeds deterministic recommendations and airing data.",
        "src-tauri/src/anilist/tests.rs":
            "Deterministic transport-level tests for AniList query constraints, response mapping, malformed data, offline and forbidden errors, and rate-limit cooldown behavior.",
        "src-tauri/src/anilist/types.rs":
            "Serde models for AniList GraphQL envelopes and raw payloads plus conversions into normalized media summaries, details, sequels, fuzzy dates, and airing schedules.",
        "src-tauri/src/commands/discovery.rs":
            "Thin typed Tauri command adapters for anime search, trending, details, recommendations, home airing, and library metadata refresh.",
        "src-tauri/src/lib.rs":
            "Rust crate entry point that initializes storage, AniList services, plugins, and the complete typed Tauri command surface before running the desktop application.",
        "src-tauri/src/recommendations/mod.rs":
            "Deterministic local recommendation ranker that derives genre and tag preferences from the library, combines similarity with trending signals, and produces human-readable explanations.",
    };
    if (roles[filePath]) return roles[filePath];

    if (filePath.includes("migrations/")) {
        const migrationRoles = {
            "0001_initial.sql": "Initial SQLite schema migration creating the authoritative library, settings, cached media, airing, and response-cache tables with supporting indexes.",
            "0002_watch_activity.sql": "Adds durable watch-activity history linked to library entries, including progress deltas, episode duration, timestamps, and query indexes.",
            "0003_activity_progress.sql": "Extends watch activity with the resulting progress value so timeline events retain the post-update library state.",
            "0004_watch_activity_identity.sql": "Rebuilds watch activity with stable event identifiers while preserving existing data and restoring its lookup indexes.",
            "0005_record_initial_watch_activity.sql": "Backfills an initial watch-activity event for eligible existing library entries without duplicating prior history.",
            "0006_media_release_date.sql": "Adds a nullable release-date field to cached media snapshots for sorting and display.",
            "0007_refresh_missing_release_dates.sql": "Expires cached media snapshots whose release date is missing so a later metadata refresh can fill it.",
        };
        return migrationRoles[basename(filePath)];
    }

    if (filePath === ".gitattributes")
        return "Git attribute rules that normalize repository text files to LF line endings.";
    if (filePath === ".prettierignore")
        return "Prettier exclusion rules that keep generated output, dependencies, build artifacts, and Rust sources out of frontend formatting.";
    if (filePath.endsWith("ambient.d.ts"))
        return "Generated SvelteKit ambient TypeScript declarations for application, platform, route, error, and page-data types.";
    if (filePath.endsWith("env.d.ts"))
        return "Generated SvelteKit environment declaration stub used by TypeScript tooling.";
    if (filePath.endsWith("non-ambient.d.ts"))
        return "Generated SvelteKit module declarations for route parameters, transport typing, and generated application modules.";
    if (filePath.endsWith("manifest.json"))
        return "Generated Vite build manifest mapping application modules and routes to their compiled JavaScript and CSS assets.";
    if (filePath.endsWith("version.json"))
        return "Generated SvelteKit client version marker used to detect deployed application updates.";
    if (filePath.endsWith(".css"))
        return "Generated minified CSS asset containing the compiled styles for a SvelteKit application route or layout.";
    if (filePath.includes("generated/client") && filePath.endsWith("app.js"))
        return "Generated SvelteKit client metadata exposing route-node loaders, server-load flags, matchers, hooks, and hash-routing configuration.";
    if (filePath.includes("generated/client") && filePath.endsWith("matchers.js"))
        return "Generated empty SvelteKit route-parameter matcher registry for the current route set.";
    if (filePath.includes("generated/client") && filePath.includes("/nodes/"))
        return "Generated SvelteKit lazy route-node loader that imports one compiled layout or page component.";
    if (filePath.endsWith("wbPk3Yxo.js"))
        return "Generated client chunk that registers SvelteKit lifecycle callbacks for navigation, URL changes, and updated-version checks.";
    if (filePath.endsWith("_layout.svelte.js"))
        return "Compiled root layout component that renders its child route content on the server.";
    if (filePath.endsWith("_layout.ts.js"))
        return "Generated root layout module declaring prerendering and server-side rendering behavior.";
    if (filePath.endsWith("/env.js"))
        return "Generated SvelteKit server environment module containing build-time path and prerender configuration values.";
    if (filePath.endsWith("manifest-full.js"))
        return "Generated full SvelteKit server manifest describing routes, nodes, assets, MIME types, hooks, and prerender metadata.";
    return `Project file ${basename(filePath)} participating in the Animi application build or runtime.`;
}

function fileTags(filePath, category) {
    if (filePath.includes(".test.")) return ["test", "component-testing", "regression"];
    if (filePath.includes("migrations/")) return ["database", "migration", "sqlite"];
    if (filePath.startsWith(".svelte-kit/")) {
        if (filePath.endsWith(".d.ts")) return ["generated", "type-definition", "sveltekit"];
        if (filePath.endsWith(".css")) return ["generated", "styles", "sveltekit"];
        if (category === "config") return ["generated", "configuration", "build-system"];
        return ["generated", "sveltekit", "build-output"];
    }
    if (filePath.endsWith(".svelte")) return ["component", "svelte", "user-interface"];
    if (filePath.includes("anilist")) return ["anilist", "graphql", "service"];
    if (filePath.includes("recommendations")) return ["recommendation", "ranking", "domain-logic"];
    if (filePath.includes("commands/")) return ["tauri-command", "ipc", "api-handler"];
    if (filePath.endsWith("lib.rs")) return ["entry-point", "tauri", "application-bootstrap"];
    if (filePath.includes("test/")) return ["test", "utility", "responsive"];
    if (filePath.startsWith(".")) return ["configuration", "repository", "tooling"];
    return ["application", "source-code", "animi"];
}

function functionSummary(filePath, name) {
    const readable = humanize(name);
    if (filePath.includes(".test." ) || filePath.endsWith("/tests.rs")) {
        if (/^(entry|item|media|detailsFor|response|services|p4Services|p6Services|discoveryProps|homeProps|indexedItem|savedEntry|recommendationsResponse|renderSettings|createStateful)/.test(name)) {
            return `Builds deterministic ${readable} fixture data or service state used by this test suite.`;
        }
        return `Verifies ${readable} behavior using deterministic fixtures without contacting the live AniList API.`;
    }
    const explicit = {
        register_commands: "Registers the complete typed Tauri command surface on an application builder.",
        select_data_root: "Selects the normal or isolated release-smoke storage root from validated runtime inputs.",
        run: "Initializes the local database, AniList service, Tauri plugins, managed state, commands, and desktop event loop.",
        rank_recommendations: "Ranks candidate anime from library-derived preferences, similarity features, and normalized trending scores while excluding existing titles.",
        build_profile: "Builds weighted genre and tag preferences from scored or progress-bearing library entries.",
        fetch_cached: "Reads a cached payload, refreshes it under the requested policy, and returns fresh, stale, or offline fallback data with explicit freshness metadata.",
        execute: "Executes a typed AniList GraphQL operation while enforcing local rate-limit windows and mapping transport, HTTP, GraphQL, and contract failures.",
    };
    if (explicit[name]) return explicit[name];
    return `Implements ${readable} behavior within ${basename(filePath)}, including the validation and state transitions required by its surrounding module.`;
}

function functionTags(filePath, name) {
    if (filePath.includes(".test.") || filePath.endsWith("/tests.rs")) return ["test", "fixture", "validation"];
    if (/validate|check|contract|normalize|map|convert|deserialize|serialize|escape/.test(name)) return ["validation", "serialization", "utility"];
    if (/search|trending|details|airing|recommend|query/.test(name)) return ["service", "anilist", "domain-operation"];
    if (/render|mount|hydrate|component|effect|reaction|source|derived/.test(name)) return ["runtime", "svelte", "state-management"];
    if (/error|issue|fatal/.test(name)) return ["error-handling", "validation", "runtime"];
    if (/register|run|select_data_root/.test(name)) return ["entry-point", "tauri", "application-bootstrap"];
    return ["function", "module-logic", "implementation"];
}

function classSummary(filePath, name) {
    if (filePath.endsWith("types.rs"))
        return `Typed AniList payload or normalized domain model represented by ${name}, with Serde-compatible fields and explicit conversion behavior where needed.`;
    if (filePath.includes(".test.") || filePath.endsWith("/tests.rs"))
        return `Deterministic ${name} test fixture used to isolate time or transport behavior.`;
    if (name === "AniListClient")
        return "Rate-limit-aware AniList GraphQL client parameterized over transport and clock implementations for deterministic testing.";
    if (name === "AniListService")
        return "Application-facing service that composes the local database with the AniList client, caching policies, stale fallbacks, and recommendation logic.";
    if (name === "Server")
        return "Generated SvelteKit server runtime that owns the manifest, hooks, environment, routing options, initialization, and request response lifecycle.";
    return `${name} encapsulates state and behavior used by ${basename(filePath)} as part of its generated or application runtime responsibilities.`;
}

function classTags(filePath, name) {
    if (filePath.endsWith("types.rs")) return ["data-model", "serialization", "anilist"];
    if (filePath.includes("test")) return ["test", "fixture", "test-double"];
    if (/Client|Transport|Clock|Service/.test(name)) return ["service", "dependency-injection", "domain-boundary"];
    if (filePath.startsWith(".svelte-kit/")) return ["generated", "runtime", "sveltekit"];
    return ["class", "domain-model", "implementation"];
}

function fileNode(meta, result) {
    const nonEmpty = result?.nonEmptyLines ?? meta.sizeLines;
    const structures = (result?.functions?.length ?? 0) + (result?.classes?.length ?? 0);
    if (meta.fileCategory === "config") {
        return {
            id: `config:${meta.path}`,
            type: "config",
            name: basename(meta.path),
            filePath: meta.path,
            summary: fileRole(meta.path),
            tags: fileTags(meta.path, meta.fileCategory),
            complexity: complexity(nonEmpty, structures),
        };
    }
    if (meta.fileCategory === "docs") {
        return {
            id: `document:${meta.path}`,
            type: "document",
            name: basename(meta.path),
            filePath: meta.path,
            summary: fileRole(meta.path),
            tags: ["documentation", "project-guide", "reference"],
            complexity: complexity(nonEmpty, structures),
        };
    }
    if (meta.fileCategory === "data" && meta.language === "sql") {
        return {
            id: `table:${meta.path}:migration`,
            type: "table",
            name: basename(meta.path),
            filePath: meta.path,
            summary: fileRole(meta.path),
            tags: fileTags(meta.path, meta.fileCategory),
            complexity: complexity(nonEmpty, structures),
            languageNotes: "Append-only SQLite migration using parameter-free DDL or deterministic data repair statements.",
        };
    }
    return {
        id: `file:${meta.path}`,
        type: "file",
        name: basename(meta.path),
        filePath: meta.path,
        summary: fileRole(meta.path),
        tags: fileTags(meta.path, meta.fileCategory),
        complexity: complexity(nonEmpty, structures),
    };
}

function analyzeBatch(batch) {
    const extraction = JSON.parse(
        fs.readFileSync(
            path.join(uaDir, "tmp", `ua-file-extract-results-${batch.batchIndex}.json`),
            "utf8",
        ),
    );
    const resultByPath = new Map(extraction.results.map((result) => [result.path, result]));
    const nodes = [];
    const edges = [];
    const nodeIds = new Set();

    const addNode = (node) => {
        if (nodeIds.has(node.id)) return;
        nodeIds.add(node.id);
        nodes.push(node);
    };
    const addEdge = (source, target, type, weight) => {
        if (source === target) return;
        const key = `${source}|${target}|${type}`;
        if (edges.some((edge) => `${edge.source}|${edge.target}|${edge.type}` === key)) return;
        edges.push({ source, target, type, direction: "forward", weight });
    };

    for (const meta of batch.files) {
        const result = resultByPath.get(meta.path);
        const parent = fileNode(meta, result);
        addNode(parent);

        if (meta.fileCategory === "code" && result) {
            const exportsByName = new Map();
            for (const item of result.exports ?? []) {
                const entries = exportsByName.get(item.name) ?? [];
                entries.push(item);
                exportsByName.set(item.name, entries);
            }

            const significantFunctions = (result.functions ?? []).filter((item) => {
                const lineCount = item.endLine - item.startLine + 1;
                return lineCount >= 10 || (exportsByName.get(item.name) ?? []).some((entry) => entry.line === item.startLine);
            });
            const seenFunctions = new Set();
            for (const item of significantFunctions) {
                if (seenFunctions.has(item.name)) continue;
                seenFunctions.add(item.name);
                const id = `function:${meta.path}:${item.name}`;
                addNode({
                    id,
                    type: "function",
                    name: item.name,
                    filePath: meta.path,
                    lineRange: [item.startLine, item.endLine],
                    summary: functionSummary(meta.path, item.name),
                    tags: functionTags(meta.path, item.name),
                    complexity: complexity(item.endLine - item.startLine + 1),
                });
                addEdge(parent.id, id, "contains", 1.0);
                if (exportsByName.has(item.name)) addEdge(parent.id, id, "exports", 0.8);
            }

            const significantClasses = (result.classes ?? []).filter((item) => {
                const lineCount = item.endLine - item.startLine + 1;
                return lineCount >= 20 || (item.methods?.length ?? 0) >= 2 || (exportsByName.get(item.name) ?? []).some((entry) => entry.line === item.startLine);
            });
            const seenClasses = new Set();
            for (const item of significantClasses) {
                if (seenClasses.has(item.name)) continue;
                seenClasses.add(item.name);
                const id = `class:${meta.path}:${item.name}`;
                addNode({
                    id,
                    type: "class",
                    name: item.name,
                    filePath: meta.path,
                    lineRange: [item.startLine, item.endLine],
                    summary: classSummary(meta.path, item.name),
                    tags: classTags(meta.path, item.name),
                    complexity: complexity(item.endLine - item.startLine + 1, item.methods?.length ?? 0),
                });
                addEdge(parent.id, id, "contains", 1.0);
                if (exportsByName.has(item.name)) addEdge(parent.id, id, "exports", 0.8);
            }

            for (const targetPath of batch.batchImportData[meta.path] ?? []) {
                addEdge(parent.id, `file:${targetPath}`, "imports", 0.7);
                if (meta.path.includes(".test.")) {
                    addEdge(parent.id, `file:${targetPath}`, "tested_by", 0.5);
                }
            }
        }

        if (meta.fileCategory === "data" && meta.language === "sql" && result) {
            for (const definition of result.definitions ?? []) {
                if (definition.kind !== "table") continue;
                const id = `table:${meta.path}:${definition.name}`;
                addNode({
                    id,
                    type: "table",
                    name: definition.name,
                    filePath: meta.path,
                    lineRange: [definition.startLine, definition.endLine],
                    summary: `Defines the ${definition.name} SQLite table with ${definition.fields.length} persisted fields for Animi's local authoritative state.`,
                    tags: ["database", "table", "schema-definition"],
                    complexity: complexity(definition.endLine - definition.startLine + 1, definition.fields.length),
                });
                addEdge(parent.id, id, "migrates", 0.7);
            }
        }
    }

    if (batch.batchIndex === 5) {
        addEdge("file:src-tauri/src/anilist/tests.rs", "file:src-tauri/src/anilist/client.rs", "tested_by", 0.5);
        addEdge("file:src-tauri/src/anilist/tests.rs", "file:src-tauri/src/anilist/service.rs", "tested_by", 0.5);
    }
    if (batch.batchIndex === 9) {
        const targets = {
            "0003_activity_progress.sql": ["table:src-tauri/migrations/0002_watch_activity.sql:watch_activity"],
            "0004_watch_activity_identity.sql": ["table:src-tauri/migrations/0002_watch_activity.sql:watch_activity"],
            "0005_record_initial_watch_activity.sql": [
                "table:src-tauri/migrations/0001_initial.sql:library_entries",
                "table:src-tauri/migrations/0002_watch_activity.sql:watch_activity",
            ],
            "0006_media_release_date.sql": ["table:src-tauri/migrations/0001_initial.sql:media_snapshots"],
            "0007_refresh_missing_release_dates.sql": ["table:src-tauri/migrations/0001_initial.sql:media_snapshots"],
        };
        for (const meta of batch.files) {
            for (const target of targets[basename(meta.path)] ?? []) {
                addEdge(`table:${meta.path}:migration`, target, "migrates", 0.7);
            }
        }
    }

    const importExpected = batch.files
        .filter((meta) => meta.fileCategory === "code")
        .reduce((count, meta) => count + (batch.batchImportData[meta.path]?.length ?? 0), 0);
    const importActual = edges.filter((edge) => edge.type === "imports").length;
    if (importExpected !== importActual) {
        throw new Error(`Batch ${batch.batchIndex}: expected ${importExpected} import edges, found ${importActual}`);
    }

    return { nodes, edges, skipped: extraction.filesSkipped ?? [] };
}

function writeParts(batch, graph) {
    const sortedFiles = [...batch.files].sort((a, b) => a.path.localeCompare(b.path));
    let parts = Math.ceil(Math.max(graph.nodes.length / 60, graph.edges.length / 120, 1));
    while (parts < sortedFiles.length) {
        const candidateChunkSize = Math.ceil(sortedFiles.length / parts);
        let withinLimits = true;
        for (let partIndex = 0; partIndex < parts; partIndex += 1) {
            const candidateFiles = new Set(
                sortedFiles
                    .slice(partIndex * candidateChunkSize, (partIndex + 1) * candidateChunkSize)
                    .map((meta) => meta.path),
            );
            const candidateNodeIds = new Set(
                graph.nodes.filter((node) => candidateFiles.has(node.filePath)).map((node) => node.id),
            );
            const candidateEdgeCount = graph.edges.filter((edge) => candidateNodeIds.has(edge.source)).length;
            if (candidateNodeIds.size > 60 || candidateEdgeCount > 120) {
                withinLimits = false;
                break;
            }
        }
        if (withinLimits) break;
        parts += 1;
    }
    const chunkSize = Math.ceil(sortedFiles.length / parts);
    const importTargets = new Set(Object.values(batch.batchImportData).flat());
    const neighborPaths = new Set(
        Object.values(batch.neighborMap ?? {})
            .flat()
            .map((neighbor) => neighbor.path),
    );

    for (let partIndex = 0; partIndex < parts; partIndex += 1) {
        const partFiles = new Set(
            sortedFiles.slice(partIndex * chunkSize, (partIndex + 1) * chunkSize).map((meta) => meta.path),
        );
        const partNodes = graph.nodes.filter((node) => partFiles.has(node.filePath));
        const partNodeIds = new Set(partNodes.map((node) => node.id));
        const partEdges = graph.edges.filter((edge) => partNodeIds.has(edge.source));
        const fragment = { nodes: partNodes, edges: partEdges };

        for (const edge of partEdges) {
            if (!partNodeIds.has(edge.source)) throw new Error(`Batch ${batch.batchIndex} part ${partIndex + 1}: missing source ${edge.source}`);
            if (partNodeIds.has(edge.target)) continue;
            const fileMatch = edge.target.match(/^file:(.+)$/);
            if (fileMatch && (importTargets.has(fileMatch[1]) || neighborPaths.has(fileMatch[1]))) continue;
            if (graph.nodes.some((node) => node.id === edge.target)) continue;
            throw new Error(`Batch ${batch.batchIndex} part ${partIndex + 1}: invalid target ${edge.target}`);
        }

        const suffix = parts === 1 ? "" : `-part-${partIndex + 1}`;
        const outputPath = path.join(uaDir, "intermediate", `batch-${batch.batchIndex}${suffix}.json`);
        fs.writeFileSync(outputPath, `${JSON.stringify(fragment, null, 4)}\n`, "utf8");
        JSON.parse(fs.readFileSync(outputPath, "utf8"));
    }
    return parts;
}

for (const batch of batchesDocument.batches) {
    if (!assigned.has(batch.batchIndex)) continue;
    const graph = analyzeBatch(batch);
    const parts = writeParts(batch, graph);
    console.log(
        JSON.stringify({
            batchIndex: batch.batchIndex,
            parts,
            nodes: graph.nodes.length,
            edges: graph.edges.length,
            skipped: graph.skipped,
        }),
    );
}
