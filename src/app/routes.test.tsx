import { describe, expect, it } from 'vitest';

import { protectedRoutes } from './routes';

type RouteLike = {
    path?: string;
    element?: {
        props?: {
            to?: string;
        };
    };
};

describe('protectedRoutes', () => {
    it('registers the browse history page', () => {
        expect(
            protectedRoutes.some(
                (route: RouteLike) => route.path === '/browse-history'
            )
        ).toBe(true);
    });

    it('redirects the charts landing route to mutual friends', () => {
        const chartsRoute = protectedRoutes.find(
            (route: RouteLike) => route.path === '/charts'
        );
        expect(chartsRoute?.element?.props?.to).toBe('/charts/mutual');
    });
});
