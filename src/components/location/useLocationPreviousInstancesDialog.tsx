import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { PreviousInstancesTableDialog } from '@/components/dialogs/PreviousInstancesTableDialog';
import gameLogRepository from '@/repositories/gameLogRepository';
import type { GameLogPreviousInstanceWorldRow } from '@/repositories/gameLogRepository';
import type { ParsedLocation } from '@/shared/utils/location';
import { normalizeString } from '@/shared/utils/string';

type PreviousInstanceDialogRow = Partial<GameLogPreviousInstanceWorldRow> & {
    createdAt?: string;
    worldId?: string;
};

type UseLocationPreviousInstancesDialogInput = {
    currentLocation: string;
    groupName: string;
    onShowPreviousInstances?: (row: PreviousInstanceDialogRow) => void;
    parsedLocation: ParsedLocation;
    worldName: string;
    worldNameHint: string;
};

export function useLocationPreviousInstancesDialog({
    currentLocation,
    groupName,
    onShowPreviousInstances,
    parsedLocation,
    worldName,
    worldNameHint
}: UseLocationPreviousInstancesDialogInput) {
    const { t } = useTranslation();
    const [previousInstancesOpen, setPreviousInstancesOpen] = useState(false);
    const [previousInstancesRows, setPreviousInstancesRows] = useState<
        PreviousInstanceDialogRow[]
    >([]);
    const [previousInstancesTitle, setPreviousInstancesTitle] =
        useState('Instance History');
    const [previousInstancesDetailsOnly, setPreviousInstancesDetailsOnly] =
        useState(false);
    const [previousInstancesLoading, setPreviousInstancesLoading] =
        useState(false);

    function showExactPreviousInstanceInfo() {
        const payload: PreviousInstanceDialogRow = {
            location: currentLocation,
            worldId: parsedLocation.worldId,
            worldName: worldName || worldNameHint,
            groupName
        };
        if (typeof onShowPreviousInstances === 'function') {
            onShowPreviousInstances(payload);
            return;
        }
        if (!currentLocation) {
            return;
        }
        setPreviousInstancesRows([
            {
                location: currentLocation,
                worldId: parsedLocation.worldId,
                worldName: worldName || worldNameHint || parsedLocation.worldId,
                groupName
            }
        ]);
        setPreviousInstancesTitle('Instance Details');
        setPreviousInstancesDetailsOnly(true);
        setPreviousInstancesOpen(true);
    }

    async function showPreviousInstances() {
        if (!currentLocation && !parsedLocation.worldId) {
            return;
        }
        if (typeof onShowPreviousInstances === 'function') {
            onShowPreviousInstances({
                location: currentLocation,
                worldId: parsedLocation.worldId,
                worldName: worldName || worldNameHint,
                groupName
            });
            return;
        }

        if (!parsedLocation.worldId || previousInstancesLoading) {
            return;
        }

        setPreviousInstancesLoading(true);
        try {
            const instances =
                await gameLogRepository.getPreviousInstancesByWorldId({
                    worldId: parsedLocation.worldId
                });
            const normalizedCurrentLocation = normalizeString(currentLocation);
            const currentInstanceRow: PreviousInstanceDialogRow = {
                location: normalizedCurrentLocation,
                worldId: parsedLocation.worldId,
                worldName: worldName || worldNameHint || parsedLocation.worldId
            };
            const nextRows = [
                ...(normalizedCurrentLocation ? [currentInstanceRow] : []),
                ...instances
            ].sort((left, right) => {
                if (normalizedCurrentLocation) {
                    if (
                        normalizeString(left?.location) ===
                        normalizedCurrentLocation
                    ) {
                        return -1;
                    }
                    if (
                        normalizeString(right?.location) ===
                        normalizedCurrentLocation
                    ) {
                        return 1;
                    }
                }
                return (
                    Date.parse(right?.created_at || '') -
                    Date.parse(left?.created_at || '')
                );
            });

            setPreviousInstancesRows(nextRows);
            setPreviousInstancesTitle(
                `Instance History - ${worldName || worldNameHint || parsedLocation.worldId}`
            );
            setPreviousInstancesDetailsOnly(false);
            setPreviousInstancesOpen(true);
        } catch (error) {
            toast.error(
                error instanceof Error
                    ? error.message
                    : t(
                          'component.location.toast.failed_to_load_instance_history'
                      )
            );
        } finally {
            setPreviousInstancesLoading(false);
        }
    }

    const previousInstancesDialog = previousInstancesOpen ? (
        <PreviousInstancesTableDialog
            open={previousInstancesOpen}
            onOpenChange={setPreviousInstancesOpen}
            title={previousInstancesTitle}
            instances={previousInstancesRows}
            variant="world"
            onRowsChange={setPreviousInstancesRows}
            detailsOnly={previousInstancesDetailsOnly}
        />
    ) : null;

    return {
        previousInstancesDialog,
        previousInstancesLoading,
        showExactPreviousInstanceInfo,
        showPreviousInstances
    };
}
