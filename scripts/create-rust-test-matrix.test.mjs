import assert from 'node:assert/strict';
import test from 'node:test';

import { createRustTestMatrix } from './create-rust-test-matrix.mjs';

function packageMetadata(id, name, manifestPath) {
    return {
        id,
        name,
        manifest_path: manifestPath
    };
}

test('selects every non-Tauri workspace package in stable order', () => {
    const metadata = {
        workspace_root: '/repo',
        workspace_members: ['application', 'shell', 'devtool', 'overlay'],
        packages: [
            packageMetadata(
                'external',
                'external-package',
                '/registry/external/Cargo.toml'
            ),
            packageMetadata(
                'application',
                'vrcx-0-application',
                '/repo/crates/application/Cargo.toml'
            ),
            packageMetadata('shell', 'vrcx-0', '/repo/src-tauri/Cargo.toml'),
            packageMetadata(
                'overlay',
                'vrcx-0-vr-overlay',
                '/repo/crates/vr-overlay/Cargo.toml'
            ),
            packageMetadata(
                'devtool',
                'vrcx-0-overlay-devtool',
                '/repo/crates/overlay-devtool/Cargo.toml'
            )
        ]
    };

    assert.deepEqual(createRustTestMatrix(metadata), [
        { package: 'vrcx-0-application', features: '' },
        { package: 'vrcx-0-overlay-devtool', features: '' },
        { package: 'vrcx-0-vr-overlay', features: 'slint-ui' }
    ]);
});

test('rejects metadata without exactly one Tauri shell', () => {
    const metadata = {
        workspace_root: '/repo',
        workspace_members: ['application'],
        packages: [
            packageMetadata(
                'application',
                'vrcx-0-application',
                '/repo/crates/application/Cargo.toml'
            )
        ]
    };

    assert.throws(
        () => createRustTestMatrix(metadata),
        /expected exactly one Tauri shell/
    );
});

test('rejects a workspace member missing from package metadata', () => {
    const metadata = {
        workspace_root: '/repo',
        workspace_members: ['shell', 'missing'],
        packages: [
            packageMetadata('shell', 'vrcx-0', '/repo/src-tauri/Cargo.toml')
        ]
    };

    assert.throws(
        () => createRustTestMatrix(metadata),
        /workspace member missing is missing from packages/
    );
});
