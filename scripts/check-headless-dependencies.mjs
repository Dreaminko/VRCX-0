import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    '..'
);
const cargoArgs = [
    'tree',
    '-p',
    'vrcx-0-headless',
    '-e',
    'normal',
    '--locked',
    '--target',
    'all',
    '--prefix',
    'none',
    '--format',
    '{p} features=[{f}]'
];

const cargoTree = spawnSync('cargo', cargoArgs, {
    cwd: repositoryRoot,
    encoding: 'utf8',
    maxBuffer: 16 * 1024 * 1024,
    windowsHide: true
});

if (cargoTree.error) {
    process.stderr.write(
        `Failed to run cargo tree: ${cargoTree.error.message}\n`
    );
    process.exit(1);
}

if (cargoTree.stderr) {
    process.stderr.write(cargoTree.stderr);
}

if (cargoTree.status !== 0) {
    if (cargoTree.stdout) {
        process.stdout.write(cargoTree.stdout);
    }
    process.exit(cargoTree.status ?? 1);
}

const forbiddenPackagePatterns = [
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
const forbiddenWindowsFeaturePattern = /direct3d|dxgi/i;
const violations = new Map();

for (const line of cargoTree.stdout.split(/\r?\n/)) {
    const match = /^(\S+)\s+.*\sfeatures=\[(.*)\]$/.exec(line);
    if (!match) {
        continue;
    }

    const [, packageName, rawFeatures] = match;
    for (const [label, pattern] of forbiddenPackagePatterns) {
        if (pattern.test(packageName)) {
            violations.set(
                `package:${packageName}:${line}`,
                `forbidden package ${packageName} (matched ${label})\n  ${line}`
            );
        }
    }

    if (/^windows(?:-|$)/i.test(packageName)) {
        const forbiddenFeatures = rawFeatures
            .split(',')
            .map((feature) => feature.trim())
            .filter((feature) => forbiddenWindowsFeaturePattern.test(feature));
        if (forbiddenFeatures.length > 0) {
            violations.set(
                `windows-features:${packageName}:${forbiddenFeatures.join(',')}`,
                `forbidden Windows graphics features on ${packageName}: ${forbiddenFeatures.join(', ')}\n  ${line}`
            );
        }
    }
}

if (violations.size > 0) {
    process.stderr.write('Headless dependency deny-list violations:\n');
    for (const violation of violations.values()) {
        process.stderr.write(`- ${violation}\n`);
    }
    process.exit(1);
}

process.stdout.write('Headless dependency deny-list check passed.\n');
