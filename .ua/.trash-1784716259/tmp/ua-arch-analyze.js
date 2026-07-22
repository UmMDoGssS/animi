import fs from "node:fs";
import path from "node:path";

const [inputPath, outputPath] = process.argv.slice(2);

if (!inputPath || !outputPath) {
    console.error("Usage: node ua-arch-analyze.js <input.json> <output.json>");
    process.exit(1);
}

try {
    const input = JSON.parse(fs.readFileSync(inputPath, "utf8"));
    const { fileNodes, importEdges, allEdges } = input;
    const normalizedPath = (value) => (value ?? "").replaceAll("\\", "/").replace(/^\.\//, "");
    const paths = fileNodes.map((node) => normalizedPath(node.filePath)).filter(Boolean);
    const splitPaths = paths.map((filePath) => filePath.split("/").filter(Boolean));
    const prefix = [];

    if (splitPaths.length > 0) {
        const shortest = Math.min(...splitPaths.map((parts) => Math.max(parts.length - 1, 0)));
        for (let index = 0; index < shortest; index += 1) {
            const segment = splitPaths[0][index];
            if (splitPaths.every((parts) => parts[index] === segment)) {
                prefix.push(segment);
            } else {
                break;
            }
        }
    }

    const directoryGroupFor = (node) => {
        const filePath = normalizedPath(node.filePath);
        const parts = filePath.split("/").filter(Boolean);
        const remainder = parts.slice(prefix.length);
        if (remainder.length > 1) {
            return remainder[0];
        }
        const name = remainder[0] ?? node.name ?? "root";
        if (/\.(test|spec)\.[^.]+$/i.test(name)) return "test";
        if (/config|\.config\./i.test(name)) return "config";
        return "root";
    };

    const directoryGroups = {};
    const nodeTypeGroups = {};
    const nodeById = new Map();
    const groupById = new Map();
    for (const node of fileNodes) {
        nodeById.set(node.id, node);
        const group = directoryGroupFor(node);
        groupById.set(node.id, group);
        (directoryGroups[group] ??= []).push(node.id);
        (nodeTypeGroups[node.type] ??= []).push(node.id);
    }

    const fileFanIn = Object.fromEntries(fileNodes.map((node) => [node.id, 0]));
    const fileFanOut = Object.fromEntries(fileNodes.map((node) => [node.id, 0]));
    const importsByGroup = {};
    const importedByGroup = {};
    const interGroupCounts = new Map();
    const pairCounts = new Map();
    const internalCounts = Object.fromEntries(Object.keys(directoryGroups).map((group) => [group, 0]));
    const involvedCounts = Object.fromEntries(Object.keys(directoryGroups).map((group) => [group, 0]));

    for (const edge of importEdges) {
        fileFanOut[edge.source] += 1;
        fileFanIn[edge.target] += 1;
        const from = groupById.get(edge.source);
        const to = groupById.get(edge.target);
        if (!from || !to) continue;
        (importsByGroup[from] ??= new Set()).add(to);
        (importedByGroup[to] ??= new Set()).add(from);
        if (from === to) {
            internalCounts[from] += 1;
            involvedCounts[from] += 1;
        } else {
            involvedCounts[from] += 1;
            involvedCounts[to] += 1;
            const key = `${from}\u0000${to}`;
            interGroupCounts.set(key, (interGroupCounts.get(key) ?? 0) + 1);
            const pair = [from, to].sort().join("\u0000");
            const direction = pairCounts.get(pair) ?? {};
            direction[key] = (direction[key] ?? 0) + 1;
            pairCounts.set(pair, direction);
        }
    }

    const directoryAdjacency = {};
    for (const group of Object.keys(directoryGroups)) {
        directoryAdjacency[group] = {
            importsFrom: [...(importsByGroup[group] ?? [])].sort(),
            importedBy: [...(importedByGroup[group] ?? [])].sort(),
        };
    }

    const crossCategoryMap = new Map();
    const nonCodeConnections = [];
    for (const edge of allEdges) {
        const sourceNode = nodeById.get(edge.source);
        const targetNode = nodeById.get(edge.target);
        if (!sourceNode || !targetNode) continue;
        if (sourceNode.type !== targetNode.type) {
            const key = `${sourceNode.type}\u0000${targetNode.type}\u0000${edge.type}`;
            crossCategoryMap.set(key, (crossCategoryMap.get(key) ?? 0) + 1);
        }
        if (sourceNode.type !== "file" || targetNode.type !== "file") {
            nonCodeConnections.push({
                source: edge.source,
                target: edge.target,
                type: edge.type,
            });
        }
    }

    const patternTable = new Map([
        ["routes", "api"], ["api", "api"], ["controllers", "api"], ["endpoints", "api"], ["handlers", "api"],
        ["services", "service"], ["core", "service"], ["lib", "service"], ["domain", "service"], ["logic", "service"], ["internal", "service"], ["signals", "service"], ["composables", "service"], ["mailers", "service"], ["jobs", "service"], ["channels", "service"],
        ["models", "data"], ["db", "data"], ["data", "data"], ["persistence", "data"], ["repository", "data"], ["entities", "data"], ["entity", "data"], ["migrations", "data"], ["sql", "data"], ["database", "data"], ["schema", "data"],
        ["components", "ui"], ["views", "ui"], ["pages", "ui"], ["ui", "ui"], ["layouts", "ui"], ["screens", "ui"],
        ["middleware", "middleware"], ["plugins", "middleware"], ["interceptors", "middleware"], ["guards", "middleware"],
        ["utils", "utility"], ["helpers", "utility"], ["common", "utility"], ["shared", "utility"], ["tools", "utility"], ["pkg", "utility"], ["templatetags", "utility"],
        ["config", "config"], ["constants", "config"], ["env", "config"], ["settings", "config"], ["management", "config"], ["commands", "config"],
        ["__tests__", "test"], ["test", "test"], ["tests", "test"], ["spec", "test"], ["specs", "test"],
        ["types", "types"], ["interfaces", "types"], ["schemas", "types"], ["contracts", "types"], ["dtos", "types"], ["dto", "types"], ["request", "types"], ["response", "types"],
        ["hooks", "hooks"], ["store", "state"], ["state", "state"], ["reducers", "state"], ["actions", "state"], ["slices", "state"],
        ["assets", "assets"], ["static", "assets"], ["public", "assets"], ["cmd", "entry"], ["bin", "entry"],
        ["docs", "documentation"], ["documentation", "documentation"], ["wiki", "documentation"],
        ["deploy", "infrastructure"], ["deployment", "infrastructure"], ["infra", "infrastructure"], ["infrastructure", "infrastructure"], ["k8s", "infrastructure"], ["kubernetes", "infrastructure"], ["helm", "infrastructure"], ["charts", "infrastructure"], ["terraform", "infrastructure"], ["tf", "infrastructure"], ["docker", "infrastructure"],
        [".github", "ci-cd"], [".gitlab", "ci-cd"], [".circleci", "ci-cd"],
    ]);
    const patternMatches = {};
    for (const group of Object.keys(directoryGroups)) {
        if (patternTable.has(group.toLowerCase())) patternMatches[group] = patternTable.get(group.toLowerCase());
    }

    const classifyFile = (node) => {
        const filePath = normalizedPath(node.filePath);
        const name = path.posix.basename(filePath);
        if (/\.(test|spec)\.[^.]+$/i.test(name) || /^test_.*\.py$/i.test(name) || /_test\.go$/i.test(name) || /(Test\.java|_spec\.rb|Test\.php|Tests\.cs)$/i.test(name)) return "test";
        if (/\.d\.ts$/i.test(name)) return "types";
        if (/^(index\.(ts|js)|__init__\.py)$/i.test(name)) return "entry";
        if (/^(manage\.py|main\.go|main\.rs|lib\.rs|Application\.java|Program\.cs|config\.ru)$/i.test(name)) return "entry";
        if (/^(wsgi|asgi)\.py$/i.test(name)) return "config";
        if (/^(Cargo\.toml|go\.mod|Gemfile|pom\.xml|build\.gradle|composer\.json)$/i.test(name)) return "config";
        if (/^Dockerfile/i.test(name) || /^docker-compose\./i.test(name) || /\.(tf|tfvars)$/i.test(name) || /^Makefile$/i.test(name)) return "infrastructure";
        if (/^\.github\/workflows\//i.test(filePath) || /(^|\/)(\.gitlab-ci\.yml|Jenkinsfile)$/i.test(filePath)) return "ci-cd";
        if (/\.sql$/i.test(name)) return "data";
        if (/\.(graphql|gql|proto)$/i.test(name)) return "types";
        if (/\.(md|rst)$/i.test(name)) return "documentation";
        return null;
    };
    const filePatternMatches = Object.fromEntries(fileNodes.map((node) => [node.id, classifyFile(node)]).filter(([, value]) => value));

    const lowerPaths = fileNodes.map((node) => normalizedPath(node.filePath));
    const infraFiles = lowerPaths.filter((filePath) => /^Dockerfile/i.test(path.posix.basename(filePath)) || /^docker-compose\./i.test(path.posix.basename(filePath)) || /(^|\/)(k8s|kubernetes|helm|charts|terraform|tf)(\/|$)/i.test(filePath) || /\.(tf|tfvars)$/i.test(filePath) || /^\.github\/workflows\//i.test(filePath) || /(^|\/)(\.gitlab-ci\.yml|Jenkinsfile)$/i.test(filePath));
    const deploymentTopology = {
        hasDockerfile: lowerPaths.some((filePath) => /^Dockerfile/i.test(path.posix.basename(filePath))),
        hasCompose: lowerPaths.some((filePath) => /^docker-compose\./i.test(path.posix.basename(filePath))),
        hasK8s: lowerPaths.some((filePath) => /(^|\/)(k8s|kubernetes|helm|charts)(\/|$)/i.test(filePath)),
        hasTerraform: lowerPaths.some((filePath) => /(^|\/)(terraform|tf)(\/|$)/i.test(filePath) || /\.(tf|tfvars)$/i.test(filePath)),
        hasCI: lowerPaths.some((filePath) => /^\.github\/workflows\//i.test(filePath) || /(^|\/)(\.gitlab-ci\.yml|Jenkinsfile)$/i.test(filePath)),
        infraFiles,
    };

    const dataPipeline = {
        schemaFiles: fileNodes.filter((node) => ["schema", "table"].includes(node.type) || /\.(sql|graphql|gql|proto|prisma)$/i.test(normalizedPath(node.filePath))).map((node) => normalizedPath(node.filePath)),
        migrationFiles: fileNodes.filter((node) => /(^|\/)migrations?\//i.test(normalizedPath(node.filePath))).map((node) => normalizedPath(node.filePath)),
        dataModelFiles: fileNodes.filter((node) => /(^|\/)(models?|entities|domain|database|repository|persistence)(\/|$)/i.test(normalizedPath(node.filePath))).map((node) => normalizedPath(node.filePath)),
        apiHandlerFiles: fileNodes.filter((node) => /(^|\/)(routes?|api|controllers?|endpoints?|handlers?|commands?)(\/|$)/i.test(normalizedPath(node.filePath))).map((node) => normalizedPath(node.filePath)),
    };

    const documentedGroups = new Set();
    for (const node of fileNodes) {
        if (node.type === "document" || /\.(md|rst)$/i.test(normalizedPath(node.filePath))) documentedGroups.add(groupById.get(node.id));
    }
    for (const edge of allEdges.filter((edge) => edge.type === "documents")) {
        const targetGroup = groupById.get(edge.target);
        if (targetGroup) documentedGroups.add(targetGroup);
    }
    const groupNames = Object.keys(directoryGroups);
    const docCoverage = {
        groupsWithDocs: documentedGroups.size,
        totalGroups: groupNames.length,
        coverageRatio: groupNames.length === 0 ? 0 : documentedGroups.size / groupNames.length,
        undocumentedGroups: groupNames.filter((group) => !documentedGroups.has(group)),
    };

    const dependencyDirection = [];
    for (const [pair, directions] of pairCounts) {
        const [a, b] = pair.split("\u0000");
        const aToB = directions[`${a}\u0000${b}`] ?? 0;
        const bToA = directions[`${b}\u0000${a}`] ?? 0;
        if (aToB > bToA) dependencyDirection.push({ dependent: a, dependsOn: b, count: aToB, reverseCount: bToA });
        if (bToA > aToB) dependencyDirection.push({ dependent: b, dependsOn: a, count: bToA, reverseCount: aToB });
    }

    const result = {
        scriptCompleted: true,
        commonPathPrefix: prefix.join("/"),
        directoryGroups,
        nodeTypeGroups,
        directoryAdjacency,
        crossCategoryEdges: [...crossCategoryMap.entries()].map(([key, count]) => {
            const [fromType, toType, edgeType] = key.split("\u0000");
            return { fromType, toType, edgeType, count };
        }),
        nonCodeConnections,
        interGroupImports: [...interGroupCounts.entries()].map(([key, count]) => {
            const [from, to] = key.split("\u0000");
            return { from, to, count };
        }),
        intraGroupDensity: Object.fromEntries(groupNames.map((group) => [group, {
            internalEdges: internalCounts[group],
            totalEdges: involvedCounts[group],
            density: involvedCounts[group] === 0 ? 0 : internalCounts[group] / involvedCounts[group],
        }])),
        patternMatches,
        filePatternMatches,
        deploymentTopology,
        dataPipeline,
        docCoverage,
        dependencyDirection,
        fileStats: {
            totalFileNodes: fileNodes.length,
            filesPerGroup: Object.fromEntries(groupNames.map((group) => [group, directoryGroups[group].length])),
            nodeTypeCounts: Object.fromEntries(Object.entries(nodeTypeGroups).map(([type, ids]) => [type, ids.length])),
        },
        fileFanIn,
        fileFanOut,
    };
    fs.writeFileSync(outputPath, `${JSON.stringify(result, null, 4)}\n`);
} catch (error) {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exit(1);
}
