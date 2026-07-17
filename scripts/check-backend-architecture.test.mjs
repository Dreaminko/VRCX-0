import assert from 'node:assert/strict';
import path from 'node:path';
import test from 'node:test';

import {
    evaluateBackendMetadata,
    evaluateBackendTauriTrees,
    evaluateHeadlessTree,
    formatViolations,
    parseCargoTree,
    parseHeadlessTree
} from './check-backend-architecture.mjs';

const fixtureRoot = path.resolve('fixture-workspace');

function dependency(name, overrides = {}) {
    return {
        name,
        kind: null,
        optional: false,
        path: null,
        target: null,
        ...overrides
    };
}

function workspaceDependency(name, overrides = {}) {
    return dependency(name, {
        path: path.join(fixtureRoot, name),
        ...overrides
    });
}

function workspacePackage(name, dependencies = [], manifestDirectory = name) {
    return {
        id: `${name} 0.1.0`,
        name,
        manifest_path: path.join(fixtureRoot, manifestDirectory, 'Cargo.toml'),
        dependencies
    };
}

function tauriShell(dependencies = []) {
    return workspacePackage('vrcx-0', dependencies, 'src-tauri');
}

function metadata(packages) {
    return {
        packages,
        workspace_members: packages.map(
            (packageMetadata) => packageMetadata.id
        ),
        workspace_root: fixtureRoot
    };
}

function parseHeadlessFixture(...dependencyLines) {
    return parseHeadlessTree(
        [
            'vrcx-0-headless v0.1.0 (fixture) features=[]',
            ...dependencyLines
        ].join('\n')
    );
}

test('healthy headless tree parses duplicate markers and stays clean', () => {
    const packages = parseHeadlessFixture(
        'image v0.25.0 features=[png, jpeg] (*)',
        'reqwest v0.13.0 features=[json]'
    );

    assert.equal(packages.length, 3);
    assert.deepEqual(evaluateHeadlessTree(packages), []);
});

test('headless deny-list catches desktop packages and Windows graphics features', () => {
    const forbiddenPackageNames = [
        'i-slint-core',
        'openvr_sys',
        'openxr-sys',
        'ash',
        'arboard',
        'vrcx-0-vr-overlay',
        'vrcx-0-application-game',
        'vrcx-0-host-desktop',
        'vrcx-0-runtime-host-desktop'
    ];
    const packages = parseHeadlessFixture(
        ...forbiddenPackageNames.map(
            (packageName) => `${packageName} v1.0.0 features=[]`
        ),
        'winapi v0.3.9 features=[d3d11]',
        'winapi v0.3.9 features=[d3d11] (*)'
    );
    const violations = evaluateHeadlessTree(packages);

    assert.equal(violations.length, forbiddenPackageNames.length + 1);
    for (const packageName of forbiddenPackageNames) {
        assert.ok(
            violations.some((violation) =>
                violation.message.includes(packageName)
            ),
            `${packageName} should be denied`
        );
    }
    assert.ok(
        violations.some((violation) => violation.message.includes('d3d11'))
    );
});

test('cargo tree parsing fails closed on malformed or rootless output', () => {
    assert.throws(
        () => parseHeadlessTree('not cargo tree output'),
        /unrecognized cargo tree line/
    );
    assert.throws(
        () => parseCargoTree('image v0.25.0 features=[]', 'vrcx-0-core'),
        /does not contain vrcx-0-core/
    );
});

test('healthy workspace edges pass and the shell may declare Tauri', () => {
    const fixture = metadata([
        tauriShell([
            workspaceDependency('vrcx-0-application'),
            dependency('tauri')
        ]),
        workspacePackage('vrcx-0-application', [
            workspaceDependency('vrcx-0-core')
        ]),
        workspacePackage('vrcx-0-core')
    ]);

    assert.deepEqual(evaluateBackendMetadata(fixture), []);
});

test('application core edges stay acyclic across application and game owners', () => {
    const fixture = metadata([
        tauriShell(),
        workspacePackage('vrcx-0-application', [
            workspaceDependency('vrcx-0-application-core')
        ]),
        workspacePackage('vrcx-0-application-core', [
            workspaceDependency('vrcx-0-core')
        ]),
        workspacePackage('vrcx-0-application-game', [
            workspaceDependency('vrcx-0-application-core')
        ]),
        workspacePackage('vrcx-0-core')
    ]);

    assert.deepEqual(evaluateBackendMetadata(fixture), []);
});

test('application core cannot depend on application or host owners', () => {
    const fixture = metadata([
        tauriShell(),
        workspacePackage('vrcx-0-application'),
        workspacePackage('vrcx-0-application-core', [
            workspaceDependency('vrcx-0-application'),
            workspaceDependency('vrcx-0-host')
        ]),
        workspacePackage('vrcx-0-host')
    ]);
    const rules = evaluateBackendMetadata(fixture).map(
        (violation) => violation.rule
    );

    assert.ok(rules.includes('dependency-direction'));
    assert.ok(rules.includes('application-host-edge'));
});

test('optional target dev edges cannot reverse desktop or host boundaries', () => {
    const fixture = metadata([
        tauriShell(),
        workspacePackage('vrcx-0-application', [
            workspaceDependency('vrcx-0-host-desktop', {
                kind: 'dev',
                optional: true,
                target: 'cfg(target_os = "windows")'
            })
        ]),
        workspacePackage('vrcx-0-host-desktop')
    ]);
    const rules = evaluateBackendMetadata(fixture).map(
        (violation) => violation.rule
    );

    assert.ok(rules.includes('dependency-direction'));
    assert.ok(rules.includes('desktop-reverse-edge'));
    assert.ok(rules.includes('application-host-edge'));
});

test('backend Tauri checks catch disabled direct and resolved transitive dependencies', () => {
    const fixture = metadata([
        tauriShell(),
        workspacePackage('vrcx-0-runtime-host', [
            dependency('tauri-plugin-dialog', {
                optional: true,
                target: 'cfg(target_os = "windows")'
            })
        ])
    ]);
    const directViolations = evaluateBackendMetadata(fixture);
    const transitiveViolations = evaluateBackendTauriTrees(
        [
            {
                rootPackage: 'vrcx-0-runtime-host',
                manifest: 'crates/runtime-host/Cargo.toml',
                packages: parseCargoTree(
                    [
                        'vrcx-0-runtime-host v0.1.0 (fixture) features=[]',
                        'wrapper v1.0.0 features=[]',
                        'tauri-plugin-dialog v2.0.0 features=[]',
                        'tauri-build v2.0.0 features=[]',
                        'tauri-build v2.0.0 features=[] (*)'
                    ].join('\n'),
                    'vrcx-0-runtime-host'
                )
            }
        ],
        new Map([['vrcx-0-runtime-host', new Set(['tauri-plugin-dialog'])]])
    );

    assert.ok(
        directViolations.some((violation) => violation.rule === 'backend-tauri')
    );
    assert.equal(transitiveViolations.length, 1);
    assert.match(transitiveViolations[0].message, /tauri-build/);
});

test('same-name path forks cannot impersonate workspace members', () => {
    const fixture = metadata([
        tauriShell(),
        workspacePackage('vrcx-0-application', [
            dependency('vrcx-0-core', {
                path: path.join(fixtureRoot, 'external-core-fork')
            })
        ]),
        workspacePackage('vrcx-0-core')
    ]);

    assert.ok(
        evaluateBackendMetadata(fixture).some((violation) =>
            violation.message.includes(
                'does not point to classified workspace member'
            )
        )
    );
});

test('unknown crates, local path edges, cycles, and overlay extras fail closed', () => {
    const fixture = metadata([
        tauriShell(),
        workspacePackage('vrcx-0-application', [
            workspaceDependency('vrcx-0-runtime-host'),
            dependency('local-helper', {
                path: path.join(fixtureRoot, 'local-helper')
            })
        ]),
        workspacePackage('vrcx-0-runtime-host', [
            workspaceDependency('vrcx-0-application')
        ]),
        workspacePackage('vrcx-0-overlay-devtool', [
            workspaceDependency('vrcx-0-vr-overlay'),
            workspaceDependency('vrcx-0-core')
        ]),
        workspacePackage('vrcx-0-vr-overlay'),
        workspacePackage('vrcx-0-core'),
        workspacePackage('vrcx-0-new-backend')
    ]);
    const violations = evaluateBackendMetadata(fixture);
    const rules = violations.map((violation) => violation.rule);

    assert.ok(rules.includes('dependency-direction'));
    assert.ok(rules.includes('workspace-cycle'));
    assert.ok(rules.includes('overlay-devtool-edge'));
    assert.ok(
        violations.some((violation) =>
            violation.message.includes('unclassified workspace package')
        )
    );
    assert.ok(
        violations.some((violation) =>
            violation.message.includes('local path dependency')
        )
    );
});

test('violation formatting is stable and concise', () => {
    const output = formatViolations([
        {
            rule: 'z-rule',
            manifest: 'z/Cargo.toml',
            message: 'last'
        },
        {
            rule: 'a-rule',
            manifest: 'a/Cargo.toml',
            message: 'first'
        }
    ]);

    assert.equal(
        output,
        'Backend architecture check failed (2 violations).\n\n[a-rule] a/Cargo.toml\n  first\n\n[z-rule] z/Cargo.toml\n  last\n'
    );
});
