// @vitest-environment jsdom

import { act, cleanup, render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    getUserMemo: vi.fn(),
    saveUserMemo: vi.fn(),
    saveUserNote: vi.fn(),
    toastError: vi.fn(),
    toastSuccess: vi.fn()
}));

vi.mock('sonner', () => ({
    toast: {
        error: mocks.toastError,
        success: mocks.toastSuccess
    }
}));

vi.mock('@/repositories/memoPersistenceRepository', () => ({
    default: {
        getUserMemo: mocks.getUserMemo,
        saveUserMemo: mocks.saveUserMemo
    }
}));

vi.mock('@/repositories/vrchatToolsRepository', () => ({
    default: {
        saveUserNote: mocks.saveUserNote
    }
}));

import { useUserDialogMemoState } from './useUserDialogMemoState';

type HookProps = Parameters<typeof useUserDialogMemoState>[0];
type HookValue = ReturnType<typeof useUserDialogMemoState>;

function HookHarness({
    onValue,
    props
}: {
    onValue: (value: HookValue) => void;
    props: HookProps;
}) {
    onValue(useUserDialogMemoState(props));
    return null;
}

describe('useUserDialogMemoState', () => {
    let current: HookValue | null;

    function createProps(): HookProps {
        return {
            activeUserTargetRef: {
                current: {
                    userId: 'usr_self',
                    endpoint: 'https://api.vrchat.cloud/api/1'
                }
            },
            applyFriendPatch: vi.fn(),
            currentEndpoint: 'https://api.vrchat.cloud/api/1',
            friendsById: {},
            normalizedUserId: 'usr_self',
            profile: {
                id: 'usr_self',
                displayName: 'Current User',
                note: 'Existing VRChat note'
            },
            setBaseProfile: vi.fn(),
            t: ((key: string) => key) as HookProps['t']
        };
    }

    beforeEach(() => {
        vi.clearAllMocks();
        current = null;
        mocks.getUserMemo.mockResolvedValue({
            userId: 'usr_self',
            memo: 'Existing local note'
        });
        mocks.saveUserMemo.mockResolvedValue({
            userId: 'usr_self',
            memo: 'Updated local note'
        });
        mocks.saveUserNote.mockResolvedValue(undefined);
    });

    afterEach(() => {
        cleanup();
    });

    function value() {
        if (!current) {
            throw new Error('Hook value is unavailable.');
        }
        return current;
    }

    async function renderMemoState() {
        render(
            <HookHarness
                onValue={(nextValue) => {
                    current = nextValue;
                }}
                props={createProps()}
            />
        );

        await waitFor(() => {
            expect(value().memo).toBe('Existing local note');
        });
    }

    function editBothNotes() {
        act(() => {
            value().editMemo();
        });
        act(() => {
            value().memoDialog.onNoteChange('Updated VRChat note');
            value().memoDialog.onMemoChange('Updated local note');
        });
    }

    async function saveNotes() {
        await act(async () => {
            await value().memoDialog.onSave();
        });
    }

    it('saves an updated VRChat note when editing the current user', async () => {
        await renderMemoState();
        editBothNotes();
        await saveNotes();

        expect(mocks.saveUserNote).toHaveBeenCalledWith({
            targetUserId: 'usr_self',
            note: 'Updated VRChat note'
        });
        expect(mocks.saveUserMemo).toHaveBeenCalledWith({
            userId: 'usr_self',
            memo: 'Updated local note'
        });
    });

    it('saves the local memo when the VRChat note save fails', async () => {
        mocks.saveUserNote.mockRejectedValueOnce(
            new Error('VRChat note save failed')
        );
        await renderMemoState();
        editBothNotes();
        await saveNotes();

        expect(mocks.saveUserMemo).toHaveBeenCalledWith({
            userId: 'usr_self',
            memo: 'Updated local note'
        });
        expect(value().memo).toBe('Updated local note');
        expect(value().memoDialog.open).toBe(true);
        expect(value().memoDialog.saving).toBe(false);
        expect(mocks.toastError).toHaveBeenCalledWith(
            'VRChat note save failed'
        );
    });

    it('does not repeat a successful VRChat note save when the local memo is retried', async () => {
        mocks.saveUserMemo.mockRejectedValueOnce(
            new Error('Local memo save failed')
        );
        await renderMemoState();
        editBothNotes();
        await saveNotes();

        expect(value().memoDialog.open).toBe(true);
        expect(value().memoDialog.saving).toBe(false);
        expect(mocks.toastError).toHaveBeenCalledWith('Local memo save failed');

        await saveNotes();

        expect(mocks.saveUserNote).toHaveBeenCalledTimes(1);
        expect(mocks.saveUserMemo).toHaveBeenCalledTimes(2);
        expect(value().memoDialog.open).toBe(false);
    });
});
