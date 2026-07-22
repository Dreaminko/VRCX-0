import { describe, expect, it } from 'vitest';

import {
    applyMutualFriendsViewFilters,
    countIsolatedMutualFriendNodes,
    MUTUAL_GRAPH_DEFAULT_VIEW_FILTERS
} from './mutualFriendsFilters';
import type { MutualFriendGraph } from './mutualFriendsTypes';

function buildNode(id: string, label: string, degree: number) {
    return { id, label, degree, lastFetchedAt: null, optedOut: false };
}

const graph: MutualFriendGraph = {
    nodes: [
        buildNode('usr_a', 'Ava', 2),
        buildNode('usr_b', 'Ben', 2),
        buildNode('usr_c', 'Cora', 1),
        buildNode('usr_d', 'Dana', 0)
    ],
    links: [
        { source: 'usr_a', target: 'usr_b' },
        { source: 'usr_b', target: 'usr_c' },
        { source: 'usr_a', target: 'usr_c' }
    ]
};

const communityIndexById = new Map([
    ['usr_a', 0],
    ['usr_b', 0],
    ['usr_c', 1],
    ['usr_d', 1]
]);

describe('applyMutualFriendsViewFilters', () => {
    it('keeps direct neighbors visible when searching for a person in the graph', () => {
        const filtered = applyMutualFriendsViewFilters(
            graph,
            { ...MUTUAL_GRAPH_DEFAULT_VIEW_FILTERS, searchQuery: 'ava' },
            communityIndexById
        );

        expect(filtered.nodes.map((node) => node.id)).toEqual([
            'usr_a',
            'usr_b',
            'usr_c'
        ]);
        expect(filtered.links).toHaveLength(3);
    });

    it('returns nothing when the search matches no one', () => {
        expect(
            applyMutualFriendsViewFilters(
                graph,
                {
                    ...MUTUAL_GRAPH_DEFAULT_VIEW_FILTERS,
                    searchQuery: 'missing'
                },
                communityIndexById
            )
        ).toEqual({ nodes: [], links: [] });
    });

    it('drops sparsely connected people once a minimum degree is set', () => {
        const filtered = applyMutualFriendsViewFilters(
            graph,
            { ...MUTUAL_GRAPH_DEFAULT_VIEW_FILTERS, minDegree: 2 },
            communityIndexById
        );

        expect(filtered.nodes.map((node) => node.id)).toEqual([
            'usr_a',
            'usr_b'
        ]);
        expect(filtered.links).toEqual([{ source: 'usr_a', target: 'usr_b' }]);
    });

    it('narrows the graph to a single circle when one is focused', () => {
        const filtered = applyMutualFriendsViewFilters(
            graph,
            { ...MUTUAL_GRAPH_DEFAULT_VIEW_FILTERS, focusedCommunity: 1 },
            communityIndexById
        );

        expect(filtered.nodes.map((node) => node.id)).toEqual([
            'usr_c',
            'usr_d'
        ]);
        expect(filtered.links).toEqual([]);
    });

    it('counts people who have no connections at all', () => {
        expect(countIsolatedMutualFriendNodes(graph)).toBe(1);
    });
});
