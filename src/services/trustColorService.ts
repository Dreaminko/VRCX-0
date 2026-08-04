import { normalizeTrustColors } from '@/shared/utils/trustColors';

const TRUST_COLOR_STYLE_ID = 'trustColor';

export function applyTrustColorClasses(value: unknown) {
    if (typeof document === 'undefined') {
        return;
    }
    const trustColors = normalizeTrustColors(value);
    document.getElementById(TRUST_COLOR_STYLE_ID)?.remove();
    const style = document.createElement('style');
    style.id = TRUST_COLOR_STYLE_ID;
    style.textContent = Object.entries(trustColors)
        .map(
            ([key, color]) =>
                `.x-tag-${key} { color: ${color} !important; border-color: ${color} !important; }`
        )
        .join(' ');
    document.head.appendChild(style);
}
