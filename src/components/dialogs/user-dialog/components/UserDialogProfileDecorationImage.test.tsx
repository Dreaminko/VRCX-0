// @vitest-environment jsdom

import { fireEvent, render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { UserDialogProfileDecorationImage } from './UserDialogProfileDecorationImage';

describe('UserDialogProfileDecorationImage', () => {
    it('renders the looping asset with a reduced-motion base fallback', () => {
        const { container } = render(
            <UserDialogProfileDecorationImage
                item={{
                    id: 'invt_frame',
                    metadata: {
                        assets: [
                            {
                                type: 'mainAnimation',
                                url: 'https://example.test/main.webp'
                            },
                            {
                                type: 'base',
                                url: 'https://example.test/base.webp'
                            }
                        ]
                    }
                }}
            />
        );

        const images = [...container.querySelectorAll('img')];
        expect(images).toHaveLength(2);
        expect(images[0]?.getAttribute('src')).toBe(
            'https://example.test/main.webp'
        );
        expect(images[0]?.classList.contains('motion-reduce:hidden')).toBe(
            true
        );
        expect(images[1]?.getAttribute('src')).toBe(
            'https://example.test/base.webp'
        );
        expect(images[1]?.classList.contains('motion-reduce:block')).toBe(true);
    });

    it('renders a base-only decoration for all motion preferences', () => {
        const { container } = render(
            <UserDialogProfileDecorationImage
                item={{
                    id: 'invt_frame',
                    metadata: {
                        assets: [
                            {
                                type: 'base',
                                url: 'https://example.test/base.webp'
                            }
                        ]
                    }
                }}
            />
        );

        const images = [...container.querySelectorAll('img')];
        expect(images).toHaveLength(1);
        expect(images[0]?.getAttribute('src')).toBe(
            'https://example.test/base.webp'
        );
        expect(images[0]?.classList.contains('hidden')).toBe(false);
    });

    it('falls back to the base asset when the looping animation fails', () => {
        const { container } = render(
            <UserDialogProfileDecorationImage
                item={{
                    id: 'invt_frame',
                    metadata: {
                        assets: [
                            {
                                type: 'mainAnimation',
                                url: 'https://example.test/main.webp'
                            },
                            {
                                type: 'base',
                                url: 'https://example.test/base.webp'
                            }
                        ]
                    }
                }}
            />
        );

        fireEvent.error(container.querySelectorAll('img')[0]);

        const images = [...container.querySelectorAll('img')];
        expect(images).toHaveLength(1);
        expect(images[0]?.getAttribute('src')).toBe(
            'https://example.test/base.webp'
        );
        expect(images[0]?.classList.contains('hidden')).toBe(false);
    });

    it('does not mount an intro-only animation as a persistent decoration', () => {
        const { container } = render(
            <UserDialogProfileDecorationImage
                item={{
                    id: 'invt_intro',
                    metadata: {
                        assets: [
                            {
                                type: 'introAnimation',
                                url: 'https://example.test/intro.webp'
                            }
                        ]
                    }
                }}
            />
        );

        expect(container.childElementCount).toBe(0);
    });
});
