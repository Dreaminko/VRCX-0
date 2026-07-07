#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const layers = new Map([
    ["vrcx-0-i18n", 0],
    ["vrcx-0-core", 0],
    ["vrcx-0-integrations", 0],
    ["vrcx-0-vr-overlay", 0],
    ["vrcx-0-media", 1],
    ["vrcx-0-vrchat-client", 1],
    ["vrcx-0-persistence", 1],
    ["vrcx-0-host", 1],
    ["vrcx-0-overlay-devtool", 1],
    ["vrcx-0-application", 2],
    ["vrcx-0-runtime-host", 3],
    ["vrcx-0-headless", 4],
    ["vrcx-0-mcp", 4],
    ["vrcx-0-harness", 5],
]);

const forbiddenEdges = new Set(["vrcx-0-application->vrcx-0-host"]);
const errors = [];

function readText(filePath) {
    return fs.readFileSync(filePath, "utf8");
}

function slash(value) {
    return value.split(path.sep).join("/");
}

function packageName(cargoToml) {
    const match = cargoToml.match(/^\s*name\s*=\s*"([^"]+)"/m);
    return match?.[1] ?? "";
}

function crateCargoTomls() {
    const cratesDir = path.join(root, "crates");
    return fs
        .readdirSync(cratesDir, { withFileTypes: true })
        .filter((entry) => entry.isDirectory())
        .map((entry) => path.join(cratesDir, entry.name, "Cargo.toml"))
        .filter((cargoToml) => fs.existsSync(cargoToml));
}

const dependencySectionNames = new Set(["dependencies", "dev-dependencies", "build-dependencies"]);

function stripTomlComment(line) {
    let quote = "";
    let escaped = false;
    for (let index = 0; index < line.length; index += 1) {
        const char = line[index];
        if (escaped) {
            escaped = false;
            continue;
        }
        if (char === "\\" && quote === '"') {
            escaped = true;
            continue;
        }
        if ((char === '"' || char === "'") && (!quote || quote === char)) {
            quote = quote ? "" : char;
            continue;
        }
        if (char === "#" && !quote) {
            return line.slice(0, index);
        }
    }
    return line;
}

function splitTomlPath(value) {
    const parts = [];
    let quote = "";
    let current = "";
    for (const char of value.trim()) {
        if ((char === '"' || char === "'") && (!quote || quote === char)) {
            quote = quote ? "" : char;
            continue;
        }
        if (char === "." && !quote) {
            parts.push(current.trim());
            current = "";
            continue;
        }
        current += char;
    }
    if (current.trim()) {
        parts.push(current.trim());
    }
    return parts;
}

function dependencySectionKind(section) {
    const parts = splitTomlPath(section);
    const dependencyIndex = parts.findIndex((part) => dependencySectionNames.has(part));
    if (dependencyIndex === -1) {
        return { isDependencySection: false, dependencyName: "" };
    }
    if (dependencyIndex === parts.length - 1) {
        return { isDependencySection: true, dependencyName: "" };
    }
    return {
        isDependencySection: false,
        dependencyName: parts[dependencyIndex + 1],
    };
}

function stringValue(value) {
    const match = value.trim().match(/^["']([^"']+)["']/);
    return match?.[1] ?? "";
}

function inlinePackageName(value) {
    const match = value.match(/(?:^|[,{]\s*)package\s*=\s*["']([^"']+)["']/);
    return match?.[1] ?? "";
}

function braceDelta(value) {
    let quote = "";
    let escaped = false;
    let delta = 0;
    for (const char of value) {
        if (escaped) {
            escaped = false;
            continue;
        }
        if (char === "\\" && quote === '"') {
            escaped = true;
            continue;
        }
        if ((char === '"' || char === "'") && (!quote || quote === char)) {
            quote = quote ? "" : char;
            continue;
        }
        if (!quote && char === "{") {
            delta += 1;
        } else if (!quote && char === "}") {
            delta -= 1;
        }
    }
    return delta;
}

function setDependency(dependencies, declaredName, packageName = "") {
    if (!declaredName) {
        return;
    }
    const existing = dependencies.get(declaredName) ?? { declaredName, packageName: "" };
    dependencies.set(declaredName, {
        declaredName,
        packageName: packageName || existing.packageName,
    });
}

function dependencyPackageNames(cargoToml) {
    const dependencies = new Map();
    let inDependencySection = false;
    let dependencyTable = null;
    let inlineDependency = null;

    function finishDependencyTable() {
        if (dependencyTable) {
            setDependency(dependencies, dependencyTable.declaredName, dependencyTable.packageName);
            dependencyTable = null;
        }
    }

    function finishInlineDependency() {
        if (inlineDependency) {
            setDependency(
                dependencies,
                inlineDependency.declaredName,
                inlinePackageName(inlineDependency.value)
            );
            inlineDependency = null;
        }
    }

    for (const rawLine of cargoToml.split(/\r?\n/)) {
        const line = stripTomlComment(rawLine);
        const sectionMatch = line.match(/^\s*\[([^\]]+)]\s*$/);
        if (sectionMatch) {
            finishInlineDependency();
            finishDependencyTable();
            const sectionKind = dependencySectionKind(sectionMatch[1]);
            inDependencySection = sectionKind.isDependencySection;
            if (sectionKind.dependencyName) {
                dependencyTable = {
                    declaredName: sectionKind.dependencyName,
                    packageName: "",
                };
            }
            continue;
        }

        if (inlineDependency) {
            inlineDependency.value += `\n${line}`;
            inlineDependency.braces += braceDelta(line);
            if (inlineDependency.braces <= 0) {
                finishInlineDependency();
            }
            continue;
        }

        if (dependencyTable) {
            const packageMatch = line.match(/^\s*package\s*=\s*(.+)$/);
            if (packageMatch) {
                dependencyTable.packageName = stringValue(packageMatch[1]);
            }
            continue;
        }

        if (!inDependencySection) {
            continue;
        }

        const assignment = line.match(/^\s*([A-Za-z0-9_.-]+)\s*=\s*(.+)$/);
        if (!assignment) {
            continue;
        }
        const keyParts = splitTomlPath(assignment[1]);
        const declaredName = keyParts[0] ?? "";
        const value = assignment[2].trim();
        if (keyParts[1] === "package") {
            setDependency(dependencies, declaredName, stringValue(value));
            continue;
        }
        setDependency(dependencies, declaredName, inlinePackageName(value));
        if (value.startsWith("{")) {
            const braces = braceDelta(value);
            if (braces > 0) {
                inlineDependency = {
                    declaredName,
                    value,
                    braces,
                };
            }
        }
    }

    finishInlineDependency();
    finishDependencyTable();

    return [...dependencies.values()].map(
        (dependency) => dependency.packageName || dependency.declaredName
    );
}

function checkCargoBoundaries() {
    for (const cargoTomlPath of crateCargoTomls()) {
        const cargoToml = readText(cargoTomlPath);
        const name = packageName(cargoToml);
        const relativePath = slash(path.relative(root, cargoTomlPath));
        const layer = layers.get(name);
        if (layer === undefined) {
            errors.push(`${relativePath}: package ${name || "(unknown)"} is missing from LAYERS.`);
            continue;
        }
        for (const dependency of dependencyPackageNames(cargoToml)) {
            if (dependency === "tauri" || dependency.startsWith("tauri-")) {
                errors.push(`${relativePath}: backend crate must not depend on tauri crates.`);
            }
            if (!dependency.startsWith("vrcx-0")) {
                continue;
            }
            const dependencyLayer = layers.get(dependency);
            if (dependencyLayer === undefined) {
                errors.push(`${relativePath}: dependency ${dependency} is missing from LAYERS.`);
                continue;
            }
            const edge = `${name}->${dependency}`;
            if (forbiddenEdges.has(edge)) {
                errors.push(`${relativePath}: forbidden dependency edge ${edge}.`);
            }
            if (dependencyLayer >= layer) {
                errors.push(
                    `${relativePath}: ${name} layer ${layer} depends on ${dependency} layer ${dependencyLayer}; dependencies must point to a lower layer.`
                );
            }
        }
    }
}

function assertContains(values, expected, label) {
    if (!values.includes(expected)) {
        throw new Error(`${label}: expected ${expected}, got ${values.join(", ")}`);
    }
}

function runParserSelfTest() {
    const dependencies = dependencyPackageNames(`
        [dependencies]
        vrcx-0-core = { path = "../core" }
        host = { package = "vrcx-0-host", path = "../host" }
        tauri_dep = { package = "tauri", version = "2" }
        vrcx-0-media.workspace = true

        [target.'cfg(windows)'.dependencies]
        runtime = { package = "vrcx-0-runtime-host", path = "../runtime-host" }

        [dependencies.app_alias]
        package = "vrcx-0-application"
        path = "../application"

        [build-dependencies]
        tauri_build = { package = "tauri-build", version = "2" }
    `);
    for (const expected of [
        "vrcx-0-core",
        "vrcx-0-host",
        "tauri",
        "vrcx-0-media",
        "vrcx-0-runtime-host",
        "vrcx-0-application",
        "tauri-build",
    ]) {
        assertContains(dependencies, expected, "dependency parser self-test");
    }
}

function walk(dir, visit) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const filePath = path.join(dir, entry.name);
        if (entry.isDirectory()) {
            walk(filePath, visit);
        } else {
            visit(filePath);
        }
    }
}

function checkPathAttributes() {
    for (const scanRoot of ["crates", "src-tauri"]) {
        walk(path.join(root, scanRoot), (filePath) => {
            if (!filePath.endsWith(".rs")) {
                return;
            }
            const source = readText(filePath);
            if (source.includes("#[path")) {
                errors.push(`${slash(path.relative(root, filePath))}: #[path] module wiring is forbidden.`);
            }
        });
    }
}

runParserSelfTest();
checkCargoBoundaries();
checkPathAttributes();

if (errors.length > 0) {
    console.error("Crate boundary check failed:");
    for (const error of errors) {
        console.error(`- ${error}`);
    }
    process.exit(1);
}

console.log("Crate boundary check passed.");
