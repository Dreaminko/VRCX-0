// @vitest-environment jsdom

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/ui/shadcn/resizable', () => ({
    ResizableHandle: ({ className }: { className?: string }) => (
        <div data-testid="resize-handle" className={className} />
    )
}));

import { DashboardResizeHandle } from './DashboardResizeHandle';

describe('DashboardResizeHandle', () => {
    afterEach(cleanup);

    it('keeps a visible divider with a larger non-layout hit target', () => {
        render(<DashboardResizeHandle />);

        const handle = screen.getByTestId('resize-handle');

        expect(handle.classList.contains('w-0.5')).toBe(true);
        expect(handle.classList.contains('after:w-2')).toBe(true);
        expect(
            handle.classList.contains('aria-[orientation=horizontal]:h-0.5')
        ).toBe(true);
        expect(
            handle.classList.contains('aria-[orientation=horizontal]:after:h-2')
        ).toBe(true);
    });
});
