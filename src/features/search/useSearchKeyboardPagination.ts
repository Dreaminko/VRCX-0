import { useEffect } from 'react';
import type { SearchPaginationState } from './searchTypes';

export function useSearchKeyboardPagination({
    pagination
}: {
    pagination: SearchPaginationState;
}) {
    useEffect(() => {
        function handleKeyDown(event: KeyboardEvent) {
            if (!event.altKey) {
                return;
            }

            if (event.key === 'ArrowLeft' && !pagination.prevDisabled) {
                event.preventDefault();
                pagination.onPrev();
            }

            if (event.key === 'ArrowRight' && !pagination.nextDisabled) {
                event.preventDefault();
                pagination.onNext();
            }
        }

        window.addEventListener('keydown', handleKeyDown);
        return () => {
            window.removeEventListener('keydown', handleKeyDown);
        };
    }, [pagination]);
}
