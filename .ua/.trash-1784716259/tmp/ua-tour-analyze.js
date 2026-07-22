import fs from "node:fs";

const [inputPath, outputPath] = process.argv.slice(2);

if (!inputPath || !outputPath) {
    console.error("Usage: node ua-tour-analyze.js <input> <output>");
    process.exit(1);
}

const normalizePath = (value) => String(value ?? "").replaceAll("\\\\", "/");

try {
    const input = JSON.parse(fs.readFileSync(inputPath, "utf8"));
    const nodes = Array.isArray(input.nodes) ? input.nodes : [];
    const edges = Array.isArray(input.edges) ? input.edges : [];
    const layers = Array.isArray(input.layers) ? input.layers : [];
    const nodeById = new Map(nodes.map((node) => [node.id, node]));
    const fanIn = new Map(nodes.map((node) => [node.id, 0]));
    const fanOut = new Map(nodes.map((node) => [node.id, 0]));

    for (const edge of edges) {
        if (nodeById.has(edge.source) && nodeById.has(edge.target)) {
            fanOut.set(edge.source, (fanOut.get(edge.source) ?? 0) + 1);
            fanIn.set(edge.target, (fanIn.get(edge.target) ?? 0) + 1);
        }
    }

    const ranked = (counts, key) =>
        nodes
            .map((node) => ({
                id: node.id,
                [key]: counts.get(node.id) ?? 0,
                name: node.name,
            }))
            .sort((a, b) => b[key] - a[key] || a.id.localeCompare(b.id));

    const fanInAll = ranked(fanIn, "fanIn");
    const fanOutAll = ranked(fanOut, "fanOut");
    const fanOutValues = [...fanOut.values()].sort((a, b) => b - a);
    const fanInValues = [...fanIn.values()].sort((a, b) => a - b);
    const highFanOutCutoff = fanOutValues[Math.max(0, Math.ceil(nodes.length * 0.1) - 1)] ?? 0;
    const lowFanInCutoff = fanInValues[Math.max(0, Math.ceil(nodes.length * 0.25) - 1)] ?? 0;
    const entryNames = new Set([
        "index.ts", "index.js", "main.ts", "main.js", "app.ts", "app.js",
        "server.ts", "server.js", "mod.rs", "main.go", "main.py", "main.rs",
        "manage.py", "app.py", "wsgi.py", "asgi.py", "run.py", "__main__.py",
        "Application.java", "Main.java", "Program.cs", "config.ru", "index.php",
        "App.swift", "Application.kt", "main.cpp", "main.c",
    ]);

    const entryPointCandidates = nodes
        .filter((node) => node.type === "file" || node.type === "document")
        .map((node) => {
            const filePath = normalizePath(node.filePath || node.name);
            const basename = filePath.split("/").at(-1) ?? node.name;
            let score = 0;
            if (node.type === "file") {
                if (entryNames.has(basename)) score += 3;
                if (filePath.split("/").length <= 2) score += 1;
                if ((fanOut.get(node.id) ?? 0) >= highFanOutCutoff) score += 1;
                if ((fanIn.get(node.id) ?? 0) <= lowFanInCutoff) score += 1;
            } else if (filePath === "README.md") {
                score += 5;
            } else if (!filePath.includes("/") && filePath.endsWith(".md")) {
                score += 2;
            }
            return { id: node.id, score, name: node.name, summary: node.summary ?? "" };
        })
        .filter((candidate) => candidate.score > 0)
        .sort((a, b) => b.score - a.score || a.id.localeCompare(b.id))
        .slice(0, 5);

    const codeEntry = entryPointCandidates.find(
        (candidate) => nodeById.get(candidate.id)?.type === "file",
    );
    const adjacency = new Map(nodes.map((node) => [node.id, []]));
    for (const edge of edges) {
        if (
            (edge.type === "imports" || edge.type === "calls") &&
            nodeById.has(edge.source) &&
            nodeById.has(edge.target)
        ) {
            adjacency.get(edge.source).push(edge.target);
        }
    }
    for (const targets of adjacency.values()) targets.sort();

    const bfsTraversal = { startNode: codeEntry?.id ?? null, order: [], depthMap: {}, byDepth: {} };
    if (codeEntry) {
        const queue = [codeEntry.id];
        bfsTraversal.depthMap[codeEntry.id] = 0;
        for (let cursor = 0; cursor < queue.length; cursor += 1) {
            const id = queue[cursor];
            const depth = bfsTraversal.depthMap[id];
            bfsTraversal.order.push(id);
            const depthKey = String(depth);
            (bfsTraversal.byDepth[depthKey] ??= []).push(id);
            for (const target of adjacency.get(id) ?? []) {
                if (!(target in bfsTraversal.depthMap)) {
                    bfsTraversal.depthMap[target] = depth + 1;
                    queue.push(target);
                }
            }
        }
    }

    const inventoryItem = (node) => ({
        id: node.id,
        name: node.name,
        type: node.type,
        summary: node.summary ?? "",
    });
    const nonCodeFiles = {
        documentation: nodes.filter((node) => node.type === "document").map(inventoryItem),
        infrastructure: nodes
            .filter((node) => ["service", "pipeline", "resource"].includes(node.type))
            .map(inventoryItem),
        data: nodes
            .filter((node) => ["table", "schema", "endpoint"].includes(node.type))
            .map(inventoryItem),
        config: nodes.filter((node) => node.type === "config").map(inventoryItem),
    };

    const mutualKinds = new Map();
    for (const edge of edges) {
        if (edge.type !== "imports" && edge.type !== "calls") continue;
        mutualKinds.set(`${edge.source}\u0000${edge.target}\u0000${edge.type}`, true);
    }
    const mutualPairs = [];
    for (const edge of edges) {
        if (
            (edge.type === "imports" || edge.type === "calls") &&
            edge.source < edge.target &&
            mutualKinds.has(`${edge.target}\u0000${edge.source}\u0000${edge.type}`)
        ) {
            mutualPairs.push([edge.source, edge.target]);
        }
    }
    const clusters = [];
    const seenSeeds = new Set();
    for (const pair of mutualPairs) {
        const seedKey = [...pair].sort().join("\u0000");
        if (seenSeeds.has(seedKey)) continue;
        const members = new Set(pair);
        let expanded = true;
        while (expanded && members.size < 5) {
            expanded = false;
            for (const node of nodes) {
                if (members.has(node.id)) continue;
                const connected = new Set();
                for (const edge of edges) {
                    if (edge.source === node.id && members.has(edge.target)) connected.add(edge.target);
                    if (edge.target === node.id && members.has(edge.source)) connected.add(edge.source);
                }
                if (connected.size >= 2) {
                    members.add(node.id);
                    expanded = true;
                    if (members.size >= 5) break;
                }
            }
        }
        const memberList = [...members].sort();
        for (let a = 0; a < memberList.length; a += 1) {
            for (let b = a + 1; b < memberList.length; b += 1) {
                seenSeeds.add(`${memberList[a]}\u0000${memberList[b]}`);
            }
        }
        const edgeCount = edges.filter(
            (edge) => members.has(edge.source) && members.has(edge.target),
        ).length;
        clusters.push({ nodes: memberList, edgeCount });
    }
    clusters.sort((a, b) => b.edgeCount - a.edgeCount || b.nodes.length - a.nodes.length);

    const nodeSummaryIndex = Object.fromEntries(
        nodes.map((node) => [
            node.id,
            { name: node.name, type: node.type, summary: node.summary ?? "" },
        ]),
    );
    const result = {
        scriptCompleted: true,
        entryPointCandidates,
        fanInRanking: fanInAll.slice(0, 20),
        fanOutRanking: fanOutAll.slice(0, 20),
        bfsTraversal,
        nonCodeFiles,
        clusters: clusters.slice(0, 10),
        layers: {
            count: layers.length,
            list: layers.map(({ id, name, description }) => ({ id, name, description })),
        },
        nodeSummaryIndex,
        totalNodes: nodes.length,
        totalEdges: edges.length,
    };
    fs.writeFileSync(outputPath, `${JSON.stringify(result, null, 2)}\n`, "utf8");
} catch (error) {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exit(1);
}
