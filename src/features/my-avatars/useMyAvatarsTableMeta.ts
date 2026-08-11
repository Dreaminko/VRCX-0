import { useRef } from 'react';

import type { MyAvatarActionHandler } from './myAvatarsTypes';

export type MyAvatarsTableMeta = {
    onAvatarAction: MyAvatarActionHandler;
};

export function useMyAvatarsTableMeta(
    onAvatarAction: MyAvatarActionHandler
): MyAvatarsTableMeta {
    const metaRef = useRef<MyAvatarsTableMeta>({
        onAvatarAction
    });
    metaRef.current.onAvatarAction = onAvatarAction;

    return metaRef.current;
}
