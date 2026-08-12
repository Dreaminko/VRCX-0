import { spawnSync } from 'node:child_process';

const rawArguments = process.argv.slice(2);
const workspace = rawArguments.includes('--workspace');
const crateNames = rawArguments.filter(
    (argument) => argument !== '--workspace'
);

if (workspace && crateNames.length > 0) {
    console.error('Do not combine --workspace with crate names.');
    process.exit(2);
}

if (!workspace && crateNames.length === 0) {
    console.error(
        'Specify one or more crate directory names, for example: npm run rust:clippy:fix -- persistence application'
    );
    process.exit(2);
}

const packageNames = crateNames.map((crateName) => {
    if (crateName === 'src-tauri' || crateName === 'vrcx-0') {
        return 'vrcx-0';
    }

    return crateName.startsWith('vrcx-0-') ? crateName : `vrcx-0-${crateName}`;
});

const selectionArguments = workspace
    ? ['--workspace']
    : packageNames.flatMap((packageName) => ['--package', packageName]);

function runClippy(extraArguments: string[]): void {
    const result = spawnSync(
        'cargo',
        ['clippy', ...selectionArguments, '--all-targets', ...extraArguments],
        { stdio: 'inherit' }
    );

    if (result.error !== undefined) {
        console.error(result.error.message);
        process.exit(1);
    }

    if (result.status !== 0) {
        process.exit(result.status ?? 1);
    }
}

runClippy(['--fix', '--allow-dirty', '--keep-going']);
runClippy(['--', '-D', 'warnings']);
