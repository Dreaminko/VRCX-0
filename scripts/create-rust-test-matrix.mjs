import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const modulePath = fileURLToPath(import.meta.url);
const repositoryRoot = path.resolve(
    path.dirname(modulePath),
    '..'
);
const tauriShellManifest = 'src-tauri/Cargo.toml';
const testFeaturesByPackage = new Map([
    ['vrcx-0-vr-overlay', 'slint-ui']
]);

function normalizeManifestPath(manifestPath, workspaceRoot) {
    return path.relative(workspaceRoot, manifestPath).replaceAll(path.sep, '/');
}

export function createRustTestMatrix(metadata) {
    if (
        !Array.isArray(metadata.packages) ||
        !Array.isArray(metadata.workspace_members) ||
        typeof metadata.workspace_root !== 'string'
    ) {
        throw new Error(
            'cargo metadata is missing packages, workspace_members, or workspace_root'
        );
    }

    const packageById = new Map(
        metadata.packages.map((packageMetadata) => [
            packageMetadata.id,
            packageMetadata
        ])
    );
    const workspacePackages = metadata.workspace_members.map((packageId) => {
        const packageMetadata = packageById.get(packageId);
        if (!packageMetadata) {
            throw new Error(
                `workspace member ${packageId} is missing from packages`
            );
        }
        return packageMetadata;
    });
    const tauriShells = workspacePackages.filter(
        (packageMetadata) =>
            normalizeManifestPath(
                packageMetadata.manifest_path,
                metadata.workspace_root
            ) === tauriShellManifest
    );
    if (tauriShells.length !== 1) {
        throw new Error(
            `expected exactly one Tauri shell at ${tauriShellManifest}, found ${tauriShells.length}`
        );
    }

    const packageNames = workspacePackages
        .filter((packageMetadata) => packageMetadata !== tauriShells[0])
        .map((packageMetadata) => packageMetadata.name);
    if (packageNames.some((packageName) => typeof packageName !== 'string')) {
        throw new Error('workspace package is missing a name');
    }
    if (new Set(packageNames).size !== packageNames.length) {
        throw new Error('workspace package names must be unique');
    }
    if (packageNames.length === 0) {
        throw new Error('no non-Tauri workspace packages found for Rust tests');
    }
    return packageNames.sort().map((packageName) => ({
        package: packageName,
        features: testFeaturesByPackage.get(packageName) ?? ''
    }));
}

function loadCargoMetadata() {
    const result = spawnSync(
        'cargo',
        ['metadata', '--locked', '--no-deps', '--format-version', '1'],
        {
            cwd: repositoryRoot,
            encoding: 'utf8',
            windowsHide: true
        }
    );
    if (result.error) {
        throw result.error;
    }
    if (result.status !== 0) {
        throw new Error(
            result.stderr.trim() || `cargo metadata exited with ${result.status}`
        );
    }
    return JSON.parse(result.stdout);
}

function main() {
    process.stdout.write(
        `${JSON.stringify(createRustTestMatrix(loadCargoMetadata()))}\n`
    );
}

if (
    process.argv[1] &&
    path.resolve(process.argv[1]) === modulePath
) {
    try {
        main();
    } catch (error) {
        const reason = error instanceof Error ? error.message : String(error);
        process.stderr.write(`Could not create Rust test matrix: ${reason}\n`);
        process.exitCode = 1;
    }
}
