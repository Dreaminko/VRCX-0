import { commands, type GroupBanImportStatus } from '@/platform/tauri/bindings';

type GroupBanImportStatusListener = (status: GroupBanImportStatus) => void;

const listeners = new Set<GroupBanImportStatusListener>();

export function handleGroupBanImportStatusEvent(
    status: GroupBanImportStatus
): void {
    for (const listener of listeners) {
        listener(status);
    }
}

export function subscribeGroupBanImportStatus(
    listener: GroupBanImportStatusListener
): () => void {
    listeners.add(listener);
    return () => {
        listeners.delete(listener);
    };
}

export function isGroupBanImportActive(status: GroupBanImportStatus): boolean {
    return status.status === 'running' || status.status === 'cancelling';
}

export function startGroupBanImport(
    groupId: string,
    userIds: string[]
): Promise<GroupBanImportStatus> {
    return commands.appGroupBanImportStart({ groupId, userIds });
}

export function getGroupBanImportStatus(): Promise<GroupBanImportStatus> {
    return commands.appGroupBanImportStatus();
}

export function cancelGroupBanImport(): Promise<GroupBanImportStatus> {
    return commands.appGroupBanImportCancel();
}
