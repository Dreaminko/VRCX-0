// @vitest-environment jsdom

import { renderHook } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => {
    const translate = (key: string) => key;
    return {
        useTranslation: () => ({ t: translate })
    };
});

vi.mock('@/state/runtimeStore', () => {
    const state = {
        auth: {
            currentUserSnapshot: null
        }
    };
    return {
        useRuntimeStore: (
            selector: (value: typeof state) => unknown
        ): unknown => selector(state)
    };
});

vi.mock('./MyAvatarsViewParts', () => ({
    AvatarActionsDropdown: () => null,
    PlatformBadges: () => null,
    SortButton: () => null,
    openAvatarDetails: vi.fn()
}));

import { MY_AVATARS_COLUMN_IDS } from '../myAvatarsState';
import type { MyAvatarActionHandler } from '../myAvatarsTypes';
import { useMyAvatarsTableMeta } from '../useMyAvatarsTableMeta';
import { useMyAvatarsColumns } from './MyAvatarsColumns';

describe('useMyAvatarsColumns', () => {
    it('keeps the columns array stable while exposing the latest action handler', () => {
        const initialAction = vi.fn<MyAvatarActionHandler>();
        const { result, rerender } = renderHook(
            ({ onAvatarAction }) => {
                const tableMeta = useMyAvatarsTableMeta(onAvatarAction);
                return {
                    columns: useMyAvatarsColumns({
                        savingTagsAvatarId: '',
                        tableMeta,
                        updatingAvatarId: '',
                        uploadingImageAvatarId: ''
                    }),
                    tableMeta
                };
            },
            {
                initialProps: {
                    onAvatarAction: initialAction
                }
            }
        );
        const initialColumns = result.current.columns;
        const nextAction = vi.fn<MyAvatarActionHandler>();

        expect(initialColumns.map((column) => column.id)).toEqual(
            MY_AVATARS_COLUMN_IDS
        );
        expect(initialColumns.map((column) => column.id)).not.toContain(
            'active'
        );
        expect(initialColumns[0]?.meta).toMatchObject({
            disableReorder: true
        });
        expect(initialColumns.at(-1)?.meta?.tableHeadClassName).toContain(
            'top-0'
        );
        expect(initialColumns.at(-1)?.meta?.tableHeadClassName).toContain(
            'right-0'
        );
        expect(initialColumns.at(-1)?.meta?.tableHeadClassName).toContain(
            'vrcx-0-table-header'
        );
        expect(initialColumns.at(-1)?.meta?.tableHeadClassName).not.toContain(
            'border-l'
        );
        expect(initialColumns.at(-1)?.meta?.tableCellClassName).not.toContain(
            'border-l'
        );

        rerender({ onAvatarAction: nextAction });

        expect(result.current.columns).toBe(initialColumns);
        expect(result.current.tableMeta.onAvatarAction).toBe(nextAction);
    });
});
