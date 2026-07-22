#!/usr/bin/env node
const fs = require("fs");

const [outputPath, gitCommitHash, analyzedFiles] = process.argv.slice(2);
const metadata = {
    lastAnalyzedAt: new Date().toISOString(),
    gitCommitHash,
    version: "1.0.0",
    analyzedFiles: Number(analyzedFiles),
};

fs.writeFileSync(outputPath, `${JSON.stringify(metadata, null, 2)}\n`);
