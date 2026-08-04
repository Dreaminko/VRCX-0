export type VrchatConfig = Record<string, unknown>;

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function parseVrchatConfig(json: string): VrchatConfig {
    if (!json) {
        return {};
    }

    const parsed: unknown = JSON.parse(json);
    if (!isRecord(parsed)) {
        throw new TypeError('VRChat configuration must be a JSON object.');
    }
    return parsed;
}

export function getConfigFieldValue(
    config: VrchatConfig,
    key: string
): string | number {
    const value = config[key];
    return typeof value === 'string' || typeof value === 'number' ? value : '';
}

export function getResolutionKey(row: {
    width?: unknown;
    height?: unknown;
}): string {
    const width = Number(row.width);
    const height = Number(row.height);
    return width > 0 && height > 0 ? `${width}x${height}` : '__default__';
}

export function applyResolution(
    config: VrchatConfig,
    keyPrefix: string,
    value: string | null
): VrchatConfig {
    if (!value || value === '__default__') {
        return {
            ...config,
            [`${keyPrefix}_width`]: '',
            [`${keyPrefix}_height`]: ''
        };
    }

    const [width, height] = value.split('x');
    return {
        ...config,
        [`${keyPrefix}_width`]: Number(width) || '',
        [`${keyPrefix}_height`]: Number(height) || ''
    };
}

export function normalizeVrchatConfigForSave(
    config: VrchatConfig
): VrchatConfig {
    const output = { ...config };
    for (const key of Object.keys(output)) {
        if (key === 'picture_output_split_by_date') {
            if (output[key]) {
                delete output[key];
            }
        } else if (output[key] === '' || output[key] === false) {
            delete output[key];
        } else if (typeof output[key] === 'string') {
            const parsed = Number.parseInt(output[key], 10);
            if (!Number.isNaN(parsed)) {
                output[key] = parsed;
            }
        }
    }
    return output;
}
