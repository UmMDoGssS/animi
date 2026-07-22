#!/usr/bin/env node
const fs = require("fs");

const [graphPath, scanPath, layersPath, tourPath, outputPath, gitCommitHash] = process.argv.slice(2);
const graph = JSON.parse(fs.readFileSync(graphPath, "utf8"));
const scan = JSON.parse(fs.readFileSync(scanPath, "utf8"));
const rawLayers = JSON.parse(fs.readFileSync(layersPath, "utf8"));
const rawTour = JSON.parse(fs.readFileSync(tourPath, "utf8"));
const layers = Array.isArray(rawLayers) ? rawLayers : rawLayers.layers || [];
const tour = (Array.isArray(rawTour) ? rawTour : rawTour.steps || []).sort(
    (left, right) => left.order - right.order,
);
const nodeIds = new Set(graph.nodes.map((node) => node.id));

for (const layer of layers) {
    layer.nodeIds = (layer.nodeIds || layer.nodes || [])
        .map((entry) => (typeof entry === "string" ? entry : entry.id))
        .filter((id) => nodeIds.has(id));
}

for (const step of tour) {
    step.description ||= step.whyItMatters;
    step.nodeIds = (step.nodeIds || step.nodesToInspect || []).filter((id) => nodeIds.has(id));
    delete step.whyItMatters;
    delete step.nodesToInspect;
}

const finalGraph = {
    version: "1.0.0",
    project: {
        name: scan.name,
        languages: scan.languages,
        frameworks: scan.frameworks,
        description: scan.description,
        analyzedAt: new Date().toISOString(),
        gitCommitHash,
    },
    nodes: graph.nodes,
    edges: graph.edges,
    layers,
    tour,
};

fs.writeFileSync(outputPath, `${JSON.stringify(finalGraph, null, 2)}\n`);
