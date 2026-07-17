import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    '..'
);
const tauriShellPackage = 'vrcx-0';
const tauriShellManifest = 'src-tauri/Cargo.toml';
const overlayDevtoolPackage = 'vrcx-0-overlay-devtool';
const expectedOverlayDevtoolDependencies = new Set(['vrcx-0-vr-overlay']);
const desktopSidePackages = new Set([
    tauriShellPackage,
    'vrcx-0-application-game',
    'vrcx-0-host-desktop',
    overlayDevtoolPackage,
    'vrcx-0-runtime-host-desktop',
    'vrcx-0-vr-overlay'
]);
const baseDataPackages = [
    'vrcx-0-core',
    'vrcx-0-i18n',
    'vrcx-0-integrations',
    'vrcx-0-media',
    'vrcx-0-persistence',
    'vrcx-0-vrchat-client'
];
const dataFoundationPackages = [
    'vrcx-0-application-activity',
    'vrcx-0-application-core',
    'vrcx-0-application-realtime',
    ...baseDataPackages
];
const gameDataPackages = [
    'vrcx-0-application-activity',
    'vrcx-0-application-core',
    ...baseDataPackages
];
const allowedWorkspaceDependencies = new Map([
    [
        tauriShellPackage,
        new Set([
            'vrcx-0-application',
            'vrcx-0-application-activity',
            'vrcx-0-application-core',
            'vrcx-0-application-game',
            'vrcx-0-application-realtime',
            'vrcx-0-core',
            'vrcx-0-harness',
            'vrcx-0-host',
            'vrcx-0-host-desktop',
            'vrcx-0-i18n',
            'vrcx-0-integrations',
            'vrcx-0-mcp',
            'vrcx-0-media',
            'vrcx-0-persistence',
            'vrcx-0-runtime-host',
            'vrcx-0-runtime-host-desktop',
            'vrcx-0-vrchat-client'
        ])
    ],
    ['vrcx-0-application', new Set(dataFoundationPackages)],
    [
        'vrcx-0-application-activity',
        new Set(['vrcx-0-application-core', ...baseDataPackages])
    ],
    ['vrcx-0-application-core', new Set(baseDataPackages)],
    ['vrcx-0-application-game', new Set(gameDataPackages)],
    [
        'vrcx-0-application-realtime',
        new Set(['vrcx-0-application-core', ...baseDataPackages])
    ],
    ['vrcx-0-core', new Set()],
    [
        'vrcx-0-harness',
        new Set([
            'vrcx-0-application',
            ...dataFoundationPackages,
            'vrcx-0-mcp',
            'vrcx-0-runtime-host'
        ])
    ],
    [
        'vrcx-0-headless',
        new Set([
            'vrcx-0-application',
            ...dataFoundationPackages,
            'vrcx-0-host',
            'vrcx-0-runtime-host'
        ])
    ],
    ['vrcx-0-host', new Set(['vrcx-0-core'])],
    [
        'vrcx-0-host-desktop',
        new Set(['vrcx-0-core', 'vrcx-0-host', 'vrcx-0-vr-overlay'])
    ],
    ['vrcx-0-i18n', new Set()],
    ['vrcx-0-integrations', new Set()],
    [
        'vrcx-0-mcp',
        new Set([
            'vrcx-0-application',
            ...dataFoundationPackages,
            'vrcx-0-runtime-host'
        ])
    ],
    ['vrcx-0-media', new Set(['vrcx-0-core'])],
    [overlayDevtoolPackage, expectedOverlayDevtoolDependencies],
    ['vrcx-0-persistence', new Set(['vrcx-0-core'])],
    [
        'vrcx-0-runtime-host',
        new Set([
            'vrcx-0-application',
            ...dataFoundationPackages,
            'vrcx-0-host'
        ])
    ],
    [
        'vrcx-0-runtime-host-desktop',
        new Set([
            'vrcx-0-application',
            ...dataFoundationPackages,
            'vrcx-0-application-game',
            'vrcx-0-host',
            'vrcx-0-host-desktop',
            'vrcx-0-runtime-host',
            'vrcx-0-vr-overlay'
        ])
    ],
    ['vrcx-0-vr-overlay', new Set()],
    ['vrcx-0-vrchat-client', new Set(['vrcx-0-core'])]
]);
const forbiddenHeadlessPackages = [
    ['slint', /(?:^|[-_])slint(?:$|[-_])/i],
    ['openvr', /(?:^|[-_])openvr(?:$|[-_])/i],
    ['openxr', /(?:^|[-_])openxr(?:$|[-_])/i],
    ['ash', /(?:^|[-_])ash(?:$|[-_])/i],
    ['arboard', /(?:^|[-_])arboard(?:$|[-_])/i],
    ['vrcx-0-vr-overlay', /^vrcx-0-vr-overlay$/i],
    ['vrcx-0-application-game', /^vrcx-0-application-game$/i],
    ['vrcx-0-host-desktop', /^vrcx-0-host-desktop$/i],
    ['vrcx-0-runtime-host-desktop', /^vrcx-0-runtime-host-desktop$/i]
];
const forbiddenWindowsFeaturePattern = /direct3d|dxgi|(?:^|[_-])d3d/i;

function createViolation(rule, manifest, message) {
    return { rule, manifest, message };
}

function isTauriPackage(packageName) {
    return /^tauri(?:$|[-_])/i.test(packageName);
}

function normalizeManifestPath(manifestPath, workspaceRoot) {
    return path.relative(workspaceRoot, manifestPath).replaceAll(path.sep, '/');
}

function extractWorkspaceMetadata(metadata) {
    if (
        !Array.isArray(metadata.packages) ||
        !Array.isArray(metadata.workspace_members)
    ) {
        throw new Error(
            'cargo metadata is missing packages or workspace_members'
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
    return {
        workspacePackages,
        workspacePackageByName: new Map(
            workspacePackages.map((packageMetadata) => [
                packageMetadata.name,
                packageMetadata
            ])
        ),
        workspacePackageNames: new Set(
            workspacePackages.map((packageMetadata) => packageMetadata.name)
        ),
        workspaceRoot: metadata.workspace_root ?? repositoryRoot
    };
}

function dependencyDescription(dependency) {
    const attributes = [dependency.kind ?? 'normal'];
    if (dependency.target) {
        attributes.push(`target=${dependency.target}`);
    }
    if (dependency.optional) {
        attributes.push('optional');
    }
    return attributes.join(', ');
}

function findWorkspaceCycle(adjacency) {
    const state = new Map();
    const stack = [];

    function visit(packageName) {
        state.set(packageName, 'visiting');
        stack.push(packageName);
        const dependencies = [...(adjacency.get(packageName) ?? [])].sort();
        for (const dependencyName of dependencies) {
            if (state.get(dependencyName) === 'visiting') {
                const cycleStart = stack.indexOf(dependencyName);
                return [...stack.slice(cycleStart), dependencyName];
            }
            if (state.get(dependencyName) !== 'visited') {
                const cycle = visit(dependencyName);
                if (cycle) {
                    return cycle;
                }
            }
        }
        stack.pop();
        state.set(packageName, 'visited');
        return null;
    }

    for (const packageName of [...adjacency.keys()].sort()) {
        if (!state.has(packageName)) {
            const cycle = visit(packageName);
            if (cycle) {
                return cycle;
            }
        }
    }
    return null;
}

export function parseCargoTree(output, expectedRootPackage) {
    const packages = [];
    for (const rawLine of output.split(/\r?\n/)) {
        const line = rawLine.trim();
        if (!line) {
            continue;
        }
        const match = /^(\S+)\s+.+?\sfeatures=\[(.*)\](?: \(\*\))?$/.exec(line);
        if (!match) {
            throw new Error(`unrecognized cargo tree line: ${line}`);
        }
        packages.push({
            name: match[1],
            features: match[2]
                .split(',')
                .map((feature) => feature.trim())
                .filter(Boolean)
        });
    }
    if (packages.length === 0) {
        throw new Error('cargo tree returned no packages');
    }
    if (
        !packages.some(
            (packageEntry) => packageEntry.name === expectedRootPackage
        )
    ) {
        throw new Error(
            `cargo tree output does not contain ${expectedRootPackage}`
        );
    }
    return packages;
}

export function parseHeadlessTree(output) {
    return parseCargoTree(output, 'vrcx-0-headless');
}

export function evaluateHeadlessTree(packages) {
    const violations = [];
    const seen = new Set();
    for (const packageEntry of packages) {
        for (const [label, pattern] of forbiddenHeadlessPackages) {
            if (pattern.test(packageEntry.name)) {
                const key = `package:${packageEntry.name}:${label}`;
                if (!seen.has(key)) {
                    seen.add(key);
                    violations.push(
                        createViolation(
                            'headless-deny-list',
                            'crates/headless/Cargo.toml',
                            `forbidden package ${packageEntry.name} matched ${label}`
                        )
                    );
                }
            }
        }

        if (/^(?:windows|winapi)(?:-|$)/i.test(packageEntry.name)) {
            const forbiddenFeatures = packageEntry.features.filter((feature) =>
                forbiddenWindowsFeaturePattern.test(feature)
            );
            const key = `windows:${packageEntry.name}:${[...forbiddenFeatures].sort().join(',')}`;
            if (forbiddenFeatures.length > 0 && !seen.has(key)) {
                seen.add(key);
                violations.push(
                    createViolation(
                        'headless-deny-list',
                        'crates/headless/Cargo.toml',
                        `forbidden Windows graphics features on ${packageEntry.name}: ${forbiddenFeatures.join(', ')}`
                    )
                );
            }
        }
    }
    return violations;
}

export function evaluateBackendMetadata(metadata) {
    const {
        workspacePackages,
        workspacePackageByName,
        workspacePackageNames,
        workspaceRoot
    } = extractWorkspaceMetadata(metadata);
    const tauriShells = workspacePackages.filter(
        (packageMetadata) =>
            normalizeManifestPath(
                packageMetadata.manifest_path,
                workspaceRoot
            ) === tauriShellManifest
    );
    if (tauriShells.length !== 1) {
        throw new Error(
            `expected exactly one Tauri shell at ${tauriShellManifest}, found ${tauriShells.length}`
        );
    }
    const adjacency = new Map();
    const violations = [];

    for (const packageMetadata of workspacePackages) {
        const sourceName = packageMetadata.name;
        const manifest = normalizeManifestPath(
            packageMetadata.manifest_path,
            workspaceRoot
        );
        const isTauriShell = manifest === tauriShellManifest;
        const allowedDependencies =
            allowedWorkspaceDependencies.get(sourceName);
        if (!allowedDependencies) {
            violations.push(
                createViolation(
                    'dependency-direction',
                    manifest,
                    `unclassified workspace package ${sourceName}; update the backend topology policy before adding it`
                )
            );
        }

        const workspaceDependencies = new Set();
        for (const dependency of packageMetadata.dependencies) {
            const dependencyName = dependency.name;
            if (!isTauriShell && isTauriPackage(dependencyName)) {
                violations.push(
                    createViolation(
                        'backend-tauri',
                        manifest,
                        `${sourceName} declares forbidden ${dependencyName} dependency (${dependencyDescription(dependency)}); only ${tauriShellPackage} may depend on Tauri`
                    )
                );
            }

            if (!workspacePackageNames.has(dependencyName)) {
                if (dependency.path) {
                    violations.push(
                        createViolation(
                            'dependency-direction',
                            manifest,
                            `local path dependency ${dependencyName} (${dependencyDescription(dependency)}) is outside the classified workspace topology`
                        )
                    );
                }
                continue;
            }
            const workspaceDependency =
                workspacePackageByName.get(dependencyName);
            const workspaceDependencyDirectory = path.dirname(
                workspaceDependency.manifest_path
            );
            if (
                !dependency.path ||
                path.relative(
                    workspaceDependencyDirectory,
                    path.resolve(dependency.path)
                ) !== ''
            ) {
                violations.push(
                    createViolation(
                        'dependency-direction',
                        manifest,
                        `${dependencyName} (${dependencyDescription(dependency)}) does not point to classified workspace member ${normalizeManifestPath(workspaceDependency.manifest_path, workspaceRoot)}`
                    )
                );
                continue;
            }
            workspaceDependencies.add(dependencyName);
            if (!adjacency.has(sourceName)) {
                adjacency.set(sourceName, new Set());
            }
            adjacency.get(sourceName).add(dependencyName);

            if (
                sourceName !== overlayDevtoolPackage &&
                allowedDependencies &&
                !allowedDependencies.has(dependencyName)
            ) {
                violations.push(
                    createViolation(
                        'dependency-direction',
                        manifest,
                        `workspace edge ${sourceName} -> ${dependencyName} (${dependencyDescription(dependency)}) is not allowed by the backend topology`
                    )
                );
            }
            if (
                !desktopSidePackages.has(sourceName) &&
                desktopSidePackages.has(dependencyName)
            ) {
                violations.push(
                    createViolation(
                        'desktop-reverse-edge',
                        manifest,
                        `${sourceName} must not depend on desktop-side package ${dependencyName}`
                    )
                );
            }
            if (
                sourceName.startsWith('vrcx-0-application') &&
                dependencyName.startsWith('vrcx-0-host')
            ) {
                violations.push(
                    createViolation(
                        'application-host-edge',
                        manifest,
                        `${sourceName} must use an application port instead of depending on ${dependencyName}`
                    )
                );
            }
        }

        if (sourceName === overlayDevtoolPackage) {
            const missingDependencies = [
                ...expectedOverlayDevtoolDependencies
            ].filter(
                (dependencyName) => !workspaceDependencies.has(dependencyName)
            );
            const extraDependencies = [...workspaceDependencies].filter(
                (dependencyName) =>
                    !expectedOverlayDevtoolDependencies.has(dependencyName)
            );
            if (
                missingDependencies.length > 0 ||
                extraDependencies.length > 0
            ) {
                violations.push(
                    createViolation(
                        'overlay-devtool-edge',
                        manifest,
                        `${overlayDevtoolPackage} workspace dependencies must be exactly vrcx-0-vr-overlay; missing: ${missingDependencies.join(', ') || 'none'}; extra: ${extraDependencies.join(', ') || 'none'}`
                    )
                );
            }
        }
    }

    for (const packageName of workspacePackageNames) {
        if (!adjacency.has(packageName)) {
            adjacency.set(packageName, new Set());
        }
    }
    const cycle = findWorkspaceCycle(adjacency);
    if (cycle) {
        const sourcePackage = workspacePackages.find(
            (packageMetadata) => packageMetadata.name === cycle[0]
        );
        violations.push(
            createViolation(
                'workspace-cycle',
                normalizeManifestPath(
                    sourcePackage.manifest_path,
                    workspaceRoot
                ),
                `workspace dependency cycle: ${cycle.join(' -> ')}`
            )
        );
    }

    return violations;
}

export function evaluateBackendTauriTrees(
    trees,
    directTauriByRoot = new Map()
) {
    const violations = [];
    for (const tree of trees) {
        const seenTauriPackages = new Set(
            directTauriByRoot.get(tree.rootPackage) ?? []
        );
        for (const packageEntry of tree.packages) {
            if (
                isTauriPackage(packageEntry.name) &&
                !seenTauriPackages.has(packageEntry.name)
            ) {
                seenTauriPackages.add(packageEntry.name);
                violations.push(
                    createViolation(
                        'backend-tauri',
                        tree.manifest,
                        `forbidden resolved Tauri dependency for ${tree.rootPackage}: ${packageEntry.name}`
                    )
                );
            }
        }
    }
    return violations;
}

export function formatViolations(violations) {
    const sorted = [...violations].sort((left, right) =>
        [left.rule, left.manifest, left.message]
            .join('\0')
            .localeCompare(
                [right.rule, right.manifest, right.message].join('\0')
            )
    );
    const noun = sorted.length === 1 ? 'violation' : 'violations';
    let output = `Backend architecture check failed (${sorted.length} ${noun}).\n`;
    for (const violation of sorted) {
        output += `\n[${violation.rule}] ${violation.manifest}\n  ${violation.message}\n`;
    }
    return output;
}

function runCargo(args) {
    const result = spawnSync('cargo', args, {
        cwd: repositoryRoot,
        encoding: 'utf8',
        maxBuffer: 32 * 1024 * 1024,
        windowsHide: true
    });
    const command = `cargo ${args.join(' ')}`;
    if (result.error) {
        throw new Error(`${command}: ${result.error.message}`);
    }
    if (result.status !== 0) {
        throw new Error(
            `${command} exited with status ${result.status ?? 'unknown'}${result.stderr ? `\n${result.stderr.trim()}` : ''}`
        );
    }
    if (result.stderr) {
        process.stderr.write(result.stderr);
    }
    return result.stdout;
}

function cargoTreeArgs(packageName, edgeKinds, allFeatures = false) {
    return [
        'tree',
        '-p',
        packageName,
        ...(allFeatures ? ['--all-features'] : []),
        '-e',
        edgeKinds,
        '--locked',
        '--target',
        'all',
        '--prefix',
        'none',
        '--format',
        '{p} features=[{f}]'
    ];
}

function runCli() {
    const args = process.argv.slice(2);
    const headlessOnly = args.length === 1 && args[0] === '--headless-only';
    if (args.length > 0 && !headlessOnly) {
        throw new Error(`unknown arguments: ${args.join(' ')}`);
    }

    const headlessTree = runCargo(cargoTreeArgs('vrcx-0-headless', 'normal'));
    const violations = evaluateHeadlessTree(parseHeadlessTree(headlessTree));

    if (!headlessOnly) {
        const metadataOutput = runCargo([
            'metadata',
            '--format-version',
            '1',
            '--locked',
            '--no-deps'
        ]);
        const metadata = JSON.parse(metadataOutput);
        violations.push(...evaluateBackendMetadata(metadata));

        const { workspacePackages, workspaceRoot } =
            extractWorkspaceMetadata(metadata);
        const backendPackages = workspacePackages
            .filter(
                (packageMetadata) =>
                    normalizeManifestPath(
                        packageMetadata.manifest_path,
                        workspaceRoot
                    ) !== tauriShellManifest
            )
            .sort((left, right) => left.name.localeCompare(right.name));
        const directTauriByRoot = new Map(
            backendPackages.map((packageMetadata) => [
                packageMetadata.name,
                new Set(
                    packageMetadata.dependencies
                        .filter((dependency) => isTauriPackage(dependency.name))
                        .map((dependency) => dependency.name)
                )
            ])
        );
        const backendTauriTrees = backendPackages.map((packageMetadata) => {
            const treeOutput = runCargo(
                cargoTreeArgs(packageMetadata.name, 'normal,build,dev', true)
            );
            return {
                rootPackage: packageMetadata.name,
                manifest: normalizeManifestPath(
                    packageMetadata.manifest_path,
                    workspaceRoot
                ),
                packages: parseCargoTree(treeOutput, packageMetadata.name)
            };
        });
        violations.push(
            ...evaluateBackendTauriTrees(backendTauriTrees, directTauriByRoot)
        );
    }

    if (violations.length > 0) {
        process.stderr.write(formatViolations(violations));
        process.exitCode = 1;
        return;
    }
    process.stdout.write(
        headlessOnly
            ? 'Headless dependency deny-list check passed.\n'
            : 'Backend architecture check passed: headless, dependency direction, backend Tauri boundary.\n'
    );
}

const isMainModule =
    process.argv[1] &&
    path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMainModule) {
    try {
        runCli();
    } catch (error) {
        const reason = error instanceof Error ? error.message : String(error);
        process.stderr.write(
            `Backend architecture check could not run: ${reason}\n`
        );
        process.exitCode = 1;
    }
}
