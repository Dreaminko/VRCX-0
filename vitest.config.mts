// @ts-nocheck
import { resolve } from 'node:path';

import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

const coverageExcludedPureConstants = [
    'src/shared/constants/accessType.ts',
    'src/shared/constants/dashboard.ts',
    'src/shared/constants/emoji.ts',
    'src/shared/constants/group.ts',
    'src/shared/constants/instance.ts',
    'src/shared/constants/language.ts',
    'src/shared/constants/link.ts',
    'src/shared/constants/moderation.ts',
    'src/shared/constants/profileBackgrounds.ts',
    'src/shared/constants/settings.ts',
    'src/shared/constants/themes.ts',
    'src/shared/constants/time.ts',
    'src/shared/constants/ui.ts',
    'src/shared/constants/user.ts',
    'src/shared/constants/world.ts'
];

export default defineConfig({
    plugins: [react()],
    resolve: {
        alias: {
            '@': resolve(import.meta.dirname, 'src')
        }
    },
    test: {
        environment: 'node',
        coverage: {
            include: ['src/**/*.{ts,tsx}'],
            exclude: [
                'src/**/*.test.{ts,tsx}',
                'src/**/*.d.ts',
                'src/localization/**',
                'src/platform/tauri/bindings.ts',
                ...coverageExcludedPureConstants
            ],
            provider: 'v8',
            reporter: ['text', 'json-summary'],
            reportsDirectory: './coverage',
            thresholds: {
                statements: 32,
                branches: 31,
                functions: 28,
                lines: 32,
                'src/app/**': {
                    statements: 7,
                    branches: 15,
                    functions: 6,
                    lines: 7
                },
                'src/components/**': {
                    statements: 20,
                    branches: 21,
                    functions: 18,
                    lines: 20
                },
                'src/domain/**': {
                    statements: 85,
                    branches: 77,
                    functions: 85,
                    lines: 85
                },
                'src/features/**': {
                    statements: 24,
                    branches: 24,
                    functions: 20,
                    lines: 24
                },
                'src/lib/**': {
                    statements: 50,
                    branches: 44,
                    functions: 46,
                    lines: 50
                },
                'src/platform/**': {
                    statements: 70,
                    branches: 71,
                    functions: 63,
                    lines: 70
                },
                'src/repositories/**': {
                    statements: 37,
                    branches: 31,
                    functions: 35,
                    lines: 37
                },
                'src/services/**': {
                    statements: 64,
                    branches: 55,
                    functions: 63,
                    lines: 64
                },
                'src/shared/**': {
                    statements: 74,
                    branches: 70,
                    functions: 77,
                    lines: 74
                },
                'src/shared/utils/**': {
                    statements: 74,
                    branches: 70,
                    functions: 79,
                    lines: 74
                },
                'src/state/**': {
                    statements: 70,
                    branches: 61,
                    functions: 73,
                    lines: 70
                },
                'src/ui/**': {
                    statements: 34,
                    branches: 27,
                    functions: 27,
                    lines: 34
                }
            }
        }
    }
});
