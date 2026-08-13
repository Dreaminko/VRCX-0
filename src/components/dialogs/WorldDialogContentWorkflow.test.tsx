// @vitest-environment jsdom

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { WorldDialogContentWorkflow } from './WorldDialogContentWorkflow';

vi.mock('react-i18next', async (importOriginal) => {
    const actual = await importOriginal<typeof import('react-i18next')>();
    return {
        ...actual,
        useTranslation: () => ({ t: (key: string) => key })
    };
});

vi.mock('react-router', async (importOriginal) => {
    const actual = await importOriginal<typeof import('react-router')>();
    return {
        ...actual,
        useNavigate: () => vi.fn()
    };
});

vi.mock('./world-dialog/useWorldDialogRuntimeState', () => ({
    useWorldDialogRuntimeState: () => ({
        closeDialog: vi.fn(),
        confirm: vi.fn(),
        currentEndpoint: 'https://api.vrchat.cloud/api/1',
        currentHomeLocation: '',
        currentUserId: 'usr_self',
        isGameRunning: false,
        prompt: vi.fn(),
        setAuthBootstrap: vi.fn(),
        showLaunchDialog: vi.fn(),
        updateEntityDialogMetadata: vi.fn()
    })
}));

vi.mock('./world-dialog/useWorldDialogData', () => ({
    useWorldDialogData: () => ({
        world: null,
        setWorld: vi.fn(),
        loadStatus: 'running',
        detail: '',
        setDetail: vi.fn(),
        memo: '',
        setMemo: vi.fn(),
        previousInstances: [],
        setPreviousInstances: vi.fn(),
        hasPersistData: false,
        setHasPersistData: vi.fn(),
        worldSideData: {},
        setWorldSideData: vi.fn(),
        newInstanceGroups: []
    })
}));

vi.mock('./world-dialog/useWorldActions', () => ({
    useWorldActions: () => ({})
}));

vi.mock('./world-dialog/useWorldInstanceActions', () => ({
    useWorldInstanceActions: () => ({})
}));

vi.mock('./world-dialog/useWorldImageUpload', () => ({
    useWorldImageUpload: () => ({})
}));

vi.mock('./world-dialog/useWorldDialogOwnerActions', () => ({
    useWorldDialogOwnerActions: () => ({})
}));

afterEach(cleanup);

describe('WorldDialogContentWorkflow', () => {
    it('shows one loading message while the world profile is loading', () => {
        render(<WorldDialogContentWorkflow worldId="wrld_test" />);

        expect(
            screen.getAllByText('dialog.world.loading.loading_world_profile')
        ).toHaveLength(1);
        expect(
            screen.queryByText(
                'dialog.world.loading.fetching_the_current_vrchat_world_snapshot_for_this_dialog'
            )
        ).toBeNull();
    });
});
