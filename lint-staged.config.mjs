const quotePath = (filePath) =>
    `"${filePath.replaceAll('\\', '/').replaceAll('"', '\\"')}"`;

export default {
    '*.{js,jsx,mjs,cjs,ts,tsx,mts,cts,json,jsonc,css,md,mdx,yml,yaml,html}':
        'oxfmt --write --no-error-on-unmatched-pattern',
    '*.rs': (files) =>
        `rustfmt --edition 2021 ${files.map(quotePath).join(' ')}`
};
