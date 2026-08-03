import { beforeEach, describe, expect, it, vi } from 'vitest';

const commandMocks = vi.hoisted(() => ({
    appVrchatFavoriteAdd: vi.fn()
}));

vi.mock('@/platform/tauri/bindings', () => ({ commands: commandMocks }));

import { addFavorite } from './vrchatFavoriteRepository';

describe('vrchatFavoriteRepository', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        commandMocks.appVrchatFavoriteAdd.mockResolvedValue({
            status: 200,
            data: { id: 'fvrt_1' }
        });
    });

    it('preserves the VRC+ world favorite type at the IPC boundary', async () => {
        await addFavorite({
            type: 'vrcPlusWorld',
            favoriteId: 'wrld_1',
            tags: 'worlds4'
        });

        expect(commandMocks.appVrchatFavoriteAdd).toHaveBeenCalledWith({
            type: 'vrcPlusWorld',
            favoriteId: 'wrld_1',
            tags: 'worlds4'
        });
    });
});
