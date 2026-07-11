import fs from 'node:fs';
import { resolve } from 'node:path';

import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import browserslist from 'browserslist';
import { browserslistToTargets } from 'lightningcss';
import { defineConfig, type Plugin } from 'vite';

const webview2BuildTarget = {
    vite: 'edge140',
    browserslist: 'Edge 140'
};
const webkitBuildTarget = {
    vite: 'safari17',
    browserslist: 'Safari 17.0'
};

function getPlatformBuildTarget() {
    switch (process.platform) {
        case 'darwin':
        case 'linux':
            return webkitBuildTarget;
        case 'win32':
        default:
            return webview2BuildTarget;
    }
}

function createReactDevtoolsStandalonePlugin(enabled: boolean): Plugin {
    return {
        name: 'vrcx-0-react-devtools-standalone',
        transformIndexHtml() {
            if (!enabled) return;

            return [
                {
                    tag: 'script',
                    attrs: {
                        src: 'http://localhost:8097'
                    },
                    injectTo: 'body-prepend'
                }
            ];
        }
    };
}

export default defineConfig(({ mode }) => {
    const tauriConf = JSON.parse(
        fs.readFileSync(
            new URL('./src-tauri/tauri.conf.json', import.meta.url),
            'utf-8'
        )
    );
    const version = tauriConf.version;
    const buildTarget = getPlatformBuildTarget();
    const enableReactDevtoolsStandalone =
        mode === 'development' && process.env.VITE_REACT_DEVTOOLS === '1';
    const macosSystemFontsEnabled = process.platform === 'darwin';

    return {
        base: '',
        plugins: [
            createReactDevtoolsStandalonePlugin(enableReactDevtoolsStandalone),
            react(),
            tailwindcss()
        ],
        resolve: {
            alias: {
                '@': resolve(import.meta.dirname, 'src')
            }
        },
        css: {
            transformer: 'lightningcss',
            lightningcss: {
                targets: browserslistToTargets(
                    browserslist(buildTarget.browserslist)
                )
            }
        },
        define: {
            VERSION: JSON.stringify(version),
            VRCX_0_BUILD_LABEL: JSON.stringify(
                process.env['VRCX_0_BUILD_LABEL'] || ''
            ),
            VRCX_0_BUILD_BADGE: JSON.stringify(
                process.env['VRCX_0_BUILD_BADGE'] || ''
            ),
            VRCX_0_MACOS_SYSTEM_FONTS_ENABLED: JSON.stringify(
                macosSystemFontsEnabled
            )
        },
        server: {
            port: 9000,
            strictPort: true
        },
        build: {
            target: buildTarget.vite,
            license: {
                fileName: 'licenses/frontend-licenses.json'
            },
            copyPublicDir: false,
            reportCompressedSize: false,
            chunkSizeWarningLimit: 3000,
            assetsInlineLimit: 0,
            rolldownOptions: {
                output: {
                    assetFileNames: 'assets/[name][extname]'
                }
            }
        }
    };
});
