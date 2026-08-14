import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { extname } from 'node:path';

const formattedExtensions = new Set([
    '.cjs',
    '.css',
    '.cts',
    '.html',
    '.js',
    '.json',
    '.jsonc',
    '.jsx',
    '.md',
    '.mdx',
    '.mjs',
    '.mts',
    '.rs',
    '.ts',
    '.tsx',
    '.yaml',
    '.yml'
]);

const changedFilesResult = spawnSync(
    'git',
    ['ls-files', '--modified', '--others', '--exclude-standard', '-z'],
    { encoding: 'buffer' }
);

if (changedFilesResult.status !== 0) {
    process.stderr.write(changedFilesResult.stderr);
    process.exit(changedFilesResult.status ?? 1);
}

const formattedFiles = changedFilesResult.stdout
    .toString('utf8')
    .split('\0')
    .filter(
        (filePath) =>
            filePath.length > 0 &&
            existsSync(filePath) &&
            !filePath.replaceAll('\\', '/').startsWith('signatures/') &&
            formattedExtensions.has(extname(filePath).toLowerCase())
    );

if (formattedFiles.length > 0) {
    const addResult = spawnSync(
        'git',
        ['add', '--pathspec-from-file=-', '--pathspec-file-nul'],
        { input: `${formattedFiles.join('\0')}\0` }
    );

    if (addResult.status !== 0) {
        process.stderr.write(addResult.stderr);
        process.exit(addResult.status ?? 1);
    }
}
