import fs from "node:fs";

const [inputPath, outputPath] = process.argv.slice(2);

if (!inputPath || !outputPath) {
    console.error("Usage: node ua-arch-prepare.js <assembled-graph.json> <output.json>");
    process.exit(1);
}

const graph = JSON.parse(fs.readFileSync(inputPath, "utf8"));
const fileLevelTypes = new Set([
    "file",
    "config",
    "document",
    "service",
    "pipeline",
    "table",
    "schema",
    "resource",
    "endpoint",
]);
const fileNodes = graph.nodes
    .filter((node) => fileLevelTypes.has(node.type))
    .map(({ id, type, name, filePath, summary, tags }) => ({
        id,
        type,
        name,
        filePath,
        summary,
        tags,
    }));
const fileNodeIds = new Set(fileNodes.map((node) => node.id));
const allEdges = graph.edges.filter(
    (edge) => fileNodeIds.has(edge.source) && fileNodeIds.has(edge.target),
);
const importEdges = allEdges.filter((edge) => edge.type === "imports");

fs.writeFileSync(
    outputPath,
    `${JSON.stringify({ fileNodes, importEdges, allEdges }, null, 4)}\n`,
);
