import fs from "node:fs";

const [resultsPath, tourPath] = process.argv.slice(2);
const results = JSON.parse(fs.readFileSync(resultsPath, "utf8"));
const tour = JSON.parse(fs.readFileSync(tourPath, "utf8"));
const allowedTypes = new Set([
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
const errors = [];

if (!Array.isArray(tour) || tour.length < 5 || tour.length > 15) {
    errors.push("Tour must contain 5-15 steps.");
}

for (const [index, step] of tour.entries()) {
    if (step.order !== index + 1) errors.push(`Step ${index + 1} has invalid order.`);
    const titleWords = typeof step.title === "string" ? step.title.trim().split(/\s+/).length : 0;
    if (titleWords < 2 || titleWords > 5) errors.push(`Step ${index + 1} title must have 2-5 words.`);
    if (typeof step.description !== "string" || step.description.trim() === "") {
        errors.push(`Step ${index + 1} has no description.`);
    }
    if (!Array.isArray(step.nodeIds) || step.nodeIds.length < 1 || step.nodeIds.length > 5) {
        errors.push(`Step ${index + 1} must have 1-5 node IDs.`);
        continue;
    }
    for (const id of step.nodeIds) {
        const node = results.nodeSummaryIndex[id];
        if (!node) errors.push(`Step ${index + 1} references missing node ${id}.`);
        else if (!allowedTypes.has(node.type)) {
            errors.push(`Step ${index + 1} references disallowed ${node.type} node ${id}.`);
        }
    }
}

if (errors.length > 0) {
    console.error(errors.join("\n"));
    process.exit(1);
}

console.log(`Validated ${tour.length} tour steps and ${tour.reduce((sum, step) => sum + step.nodeIds.length, 0)} node references.`);
