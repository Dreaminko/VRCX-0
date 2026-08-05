// @vitest-environment jsdom

import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Sheet, SheetContent, SheetTitle } from './sheet';

describe('Sheet animation boundary', () => {
    it('clips the translated popup inside the application viewport', () => {
        render(
            <Sheet open>
                <SheetContent side="right">
                    <SheetTitle>Title</SheetTitle>
                </SheetContent>
            </Sheet>
        );

        const viewport = document.querySelector('[data-slot="sheet-viewport"]');
        const popup = document.querySelector('[data-slot="sheet-content"]');

        expect(viewport).toBeTruthy();
        expect(popup).toBeTruthy();
        expect(viewport?.getAttribute('role')).toBe('presentation');
        expect(viewport?.classList.contains('overflow-hidden')).toBe(true);
        expect(viewport?.contains(popup)).toBe(true);
        expect(popup?.classList.contains('absolute')).toBe(true);
        expect(popup?.className).toContain(
            'data-[side=right]:data-starting-style:translate-x-[2.5rem]'
        );
    });
});
