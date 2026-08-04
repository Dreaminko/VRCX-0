// @vitest-environment jsdom

import { afterEach, describe, expect, it } from 'vitest';

import { applyTrustColorClasses } from './trustColorService';

describe('applyTrustColorClasses', () => {
    afterEach(() => {
        document.head.replaceChildren();
    });

    it('renders normalized trust colors and replaces the previous style', () => {
        applyTrustColorClasses({ basic: '#123456' });
        const firstStyle = document.getElementById('trustColor');

        expect(firstStyle?.textContent).toContain(
            '.x-tag-basic { color: #123456 !important;'
        );

        applyTrustColorClasses({ basic: '#abcdef' });
        const replacement = document.getElementById('trustColor');
        expect(replacement).not.toBe(firstStyle);
        expect(replacement?.textContent).toContain('#ABCDEF');
        expect(document.querySelectorAll('#trustColor')).toHaveLength(1);
    });
});
