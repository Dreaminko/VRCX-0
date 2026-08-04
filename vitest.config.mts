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
                statements: 32.39,
                branches: 30.82,
                functions: 28.45,
                lines: 32.65,
                'src/app/**': {
                    statements: 7.2,
                    branches: 15.38,
                    functions: 6.25,
                    lines: 7.62
                },
                'src/components/**': {
                    statements: 20.62,
                    branches: 21.91,
                    functions: 18.22,
                    lines: 20.74
                },
                'src/domain/**': {
                    statements: 85.82,
                    branches: 77.84,
                    functions: 85.26,
                    lines: 85.78
                },
                'src/features/**': {
                    statements: 24.54,
                    branches: 24.28,
                    functions: 20.97,
                    lines: 24.86
                },
                'src/lib/**': {
                    statements: 50.32,
                    branches: 44.05,
                    functions: 46.42,
                    lines: 50.66
                },
                'src/platform/**': {
                    statements: 70.52,
                    branches: 71.28,
                    functions: 63.63,
                    lines: 70.52
                },
                'src/repositories/**': {
                    statements: 37.33,
                    branches: 31.4,
                    functions: 35.7,
                    lines: 37.39
                },
                'src/services/**': {
                    statements: 64.15,
                    branches: 55.87,
                    functions: 63.61,
                    lines: 64.19
                },
                'src/shared/**': {
                    statements: 74.69,
                    branches: 70.1,
                    functions: 77.4,
                    lines: 74.77
                },
                'src/shared/utils/**': {
                    statements: 74.8,
                    branches: 70.78,
                    functions: 79.65,
                    lines: 74.87
                },
                'src/state/**': {
                    statements: 70.23,
                    branches: 61.68,
                    functions: 73.86,
                    lines: 70.08
                },
                'src/ui/**': {
                    statements: 34.62,
                    branches: 27.38,
                    functions: 27.98,
                    lines: 34.79
                }
            }
        }
    }
});
