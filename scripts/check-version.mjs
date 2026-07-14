#!/usr/bin/env node
import { readFileSync } from "node:fs";

const packageJson = JSON.parse(readFileSync("package.json", "utf8"));
const tauriConf = JSON.parse(
  readFileSync("src-tauri/tauri.conf.json", "utf8"),
);
const cargoToml = readFileSync("src-tauri/Cargo.toml", "utf8");
const cargoMatch = cargoToml.match(/^version = "([^"]+)"/m);

if (!cargoMatch) {
  console.error("Could not read version from src-tauri/Cargo.toml");
  process.exit(1);
}

const versions = {
  "package.json": packageJson.version,
  "src-tauri/tauri.conf.json": tauriConf.version,
  "src-tauri/Cargo.toml": cargoMatch[1],
};

const unique = [...new Set(Object.values(versions))];
if (unique.length !== 1) {
  console.error("Version mismatch:");
  for (const [file, version] of Object.entries(versions)) {
    console.error(`  ${file}: ${version}`);
  }
  process.exit(1);
}

console.log(`Version OK: ${unique[0]}`);
