import { existsSync } from "node:fs";
import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";

type JsonObject = Record<string, unknown>;

const root = process.cwd();
const failures: string[] = [];

function fail(message: string) {
  failures.push(message);
}

async function readJson(path: string): Promise<JsonObject> {
  return JSON.parse(await readFile(join(root, path), "utf8")) as JsonObject;
}

function object(value: unknown, label: string): JsonObject {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    fail(`${label} must be an object.`);
    return {};
  }
  return value as JsonObject;
}

async function collectFiles(directory: string): Promise<string[]> {
  const ignored = new Set([".git", "build", "node_modules", "target"]);
  const files: string[] = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (ignored.has(entry.name)) continue;
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await collectFiles(path)));
    else files.push(path);
  }
  return files;
}

const packageJson = await readJson("package.json");
const tauriConfig = await readJson("src-tauri/tauri.conf.json");
const capability = await readJson("src-tauri/capabilities/default.json");
const cargoToml = await readFile(join(root, "src-tauri/Cargo.toml"), "utf8");
const ipcClient = await readFile(join(root, "src/lib/ipc/client.ts"), "utf8");
const anilistModule = await readFile(
  join(root, "src-tauri/src/anilist/mod.rs"),
  "utf8",
);

const expectedVersion = "1.0.0";
if (packageJson.version !== expectedVersion) fail("package.json is not 1.0.0.");
if (tauriConfig.version !== expectedVersion) fail("Tauri is not 1.0.0.");
if (
  !new RegExp(
    `^version = "${expectedVersion.replaceAll(".", "\\.")}"$`,
    "m",
  ).test(cargoToml)
) {
  fail("Cargo is not 1.0.0.");
}
if (packageJson.private !== true)
  fail("The frontend package must remain private.");
if (packageJson.license !== "UNLICENSED")
  fail("The frontend package must remain unlicensed for distribution.");
if (!/^publish = false$/m.test(cargoToml))
  fail("The Rust package must disable publishing.");

if (tauriConfig.identifier !== "app.animi.desktop")
  fail("The stable application identifier changed.");
const app = object(tauriConfig.app, "app");
const security = object(app.security, "app.security");
if (typeof security.csp !== "string" || security.csp.trim().length === 0) {
  fail("A production CSP is required.");
}
const bundle = object(tauriConfig.bundle, "bundle");
if (JSON.stringify(bundle.targets) !== JSON.stringify(["nsis"])) {
  fail("Only the Windows NSIS target may be enabled for v1.");
}
if (bundle.createUpdaterArtifacts !== false)
  fail("Updater artifacts must remain disabled.");
const windows = object(bundle.windows, "bundle.windows");
if (windows.allowDowngrades !== false)
  fail("Installer downgrades must be blocked.");
const nsis = object(windows.nsis, "bundle.windows.nsis");
if (nsis.installMode !== "currentUser")
  fail("NSIS must use current-user installation.");

if (
  JSON.stringify(capability.permissions) !== JSON.stringify(["core:default"])
) {
  fail("The main-window capability expanded beyond core defaults.");
}
if (/\b(fetch|XMLHttpRequest|WebSocket)\s*\(/.test(ipcClient)) {
  fail("The frontend IPC adapter contains a direct network primitive.");
}
if (/\b(localStorage|sessionStorage|indexedDB)\b/.test(ipcClient)) {
  fail("The frontend IPC adapter contains durable browser storage.");
}
if (!anilistModule.includes('"https://graphql.anilist.co"')) {
  fail("The approved AniList GraphQL endpoint is missing.");
}

const allFiles = await collectFiles(root);
const forbiddenLockfiles = new Set([
  "package-lock.json",
  "npm-shrinkwrap.json",
  "pnpm-lock.yaml",
  "yarn.lock",
]);
for (const path of allFiles) {
  const normalized = relative(root, path).replaceAll("\\", "/");
  const name = normalized.split("/").at(-1) ?? "";
  if (forbiddenLockfiles.has(name))
    fail(`Non-Bun lockfile found: ${normalized}`);
  if (/\.(env|pem|pfx|p12|key)$/i.test(name))
    fail(`Sensitive artifact found: ${normalized}`);
  if (
    /\.(db|sqlite|sqlite3)$/i.test(name) &&
    !normalized.startsWith("src-tauri/tests/fixtures/database/")
  ) {
    fail(`Unexpected database artifact found: ${normalized}`);
  }
}
for (const required of [
  "bun.lock",
  "src-tauri/Cargo.lock",
  "LICENSE",
  "docs/RELEASE_NOTES_1.0.0.md",
]) {
  if (!existsSync(join(root, required)))
    fail(`Required release file is missing: ${required}`);
}

if (failures.length > 0) {
  for (const failure of failures) console.error(`FAIL: ${failure}`);
  process.exit(1);
}

console.log(
  "Release audit passed: private scope, identity, CSP, NSIS policy, capabilities, URLs, lockfiles, and artifact hygiene are intact.",
);
