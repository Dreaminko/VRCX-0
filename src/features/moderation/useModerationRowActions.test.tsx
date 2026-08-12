// @vitest-environment jsdom

import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { mocks, runtimeState } = vi.hoisted(() => ({
    mocks: {
        confirm: vi.fn(),
        refreshModerationSync: vi.fn(),
        updateModerationSync: vi.fn()
    },
    runtimeState: {
        auth: {
            currentUserId: 'usr_self',
            currentUserEndpoint: 'https://api.example.test/api/1'
        }
    }
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({ t: (key: string) => key })
}));

vi.mock('@/services/dialogService', () => ({
    openUserDialog: vi.fn()
}));

vi.mock('@/services/moderationSyncService', () => ({
    refreshModerationSync: mocks.refreshModerationSync,
    updateModerationSync: mocks.updateModerationSync
}));

vi.mock('@/state/modalStore', () => ({
    useModalStore: (
        selector: (state: { confirm: typeof mocks.confirm }) => unknown
    ) => selector({ confirm: mocks.confirm })
}));

vi.mock('@/state/runtimeStore', () => ({
    useRuntimeStore: Object.assign(
        (selector: (state: typeof runtimeState) => unknown) =>
            selector(runtimeState),
        { getState: () => runtimeState }
    )
}));

import { useModerationRowActions } from './useModerationRowActions';

describe('useModerationRowActions', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        runtimeState.auth.currentUserId = 'usr_self';
        runtimeState.auth.currentUserEndpoint =
            'https://api.example.test/api/1';
    });

    afterEach(() => {
        cleanup();
    });

    it('does not apply an old row after the endpoint changes during confirmation', async () => {
        let resolveConfirmation!: (result: { ok: boolean }) => void;
        mocks.confirm.mockReturnValueOnce(
            new Promise<{ ok: boolean }>((resolve) => {
                resolveConfirmation = resolve;
            })
        );
        const { result } = renderHook(() =>
            useModerationRowActions({
                rows: [],
                setDetail: vi.fn(),
                setRows: vi.fn()
            })
        );

        await act(async () => {
            const action = result.current.handleDeleteModeration({
                sourceUserId: 'usr_self',
                targetUserId: 'usr_target',
                targetDisplayName: 'Target User',
                type: 'block'
            });
            runtimeState.auth.currentUserEndpoint =
                'https://api.other.test/api/1';
            resolveConfirmation({ ok: true });
            await action;
        });

        expect(mocks.updateModerationSync).not.toHaveBeenCalled();
        expect(mocks.refreshModerationSync).not.toHaveBeenCalled();
    });
});
