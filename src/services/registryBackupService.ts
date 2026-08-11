import { commands } from '@/platform/tauri/bindings';
import type { RegistryBackupSnapshot } from '@/platform/tauri/bindings';

import { requireHostCapability } from './hostCapabilityService';

async function listVrcRegistryBackups(): Promise<RegistryBackupSnapshot[]> {
    requireHostCapability('registryPrefs');
    return commands.appRegistryBackupList();
}

async function backupVrcRegistry(
    name: string = 'Manual Backup'
): Promise<RegistryBackupSnapshot[]> {
    requireHostCapability('registryPrefs');
    return commands.appRegistryBackupCreate(name);
}

async function restoreVrcRegistryBackup(
    key: string
): Promise<RegistryBackupSnapshot> {
    requireHostCapability('registryPrefs');
    return commands.appRegistryBackupRestore(key);
}

async function saveVrcRegistryBackupToFile(key: string): Promise<string> {
    requireHostCapability('registryPrefs');
    return commands.appRegistryBackupExportToFile(key);
}

async function restoreVrcRegistryBackupFromFile(): Promise<boolean> {
    requireHostCapability('registryPrefs');
    return commands.appRegistryBackupImportFromFile();
}

async function deleteVrcRegistryFolder(): Promise<unknown> {
    requireHostCapability('registryPrefs');
    return commands.appDeleteVrchatRegistryFolder();
}

async function deleteVrcRegistryBackup(
    key: string
): Promise<RegistryBackupSnapshot[]> {
    requireHostCapability('registryPrefs');
    return commands.appRegistryBackupDelete(key);
}

export {
    backupVrcRegistry,
    deleteVrcRegistryBackup,
    deleteVrcRegistryFolder,
    listVrcRegistryBackups,
    restoreVrcRegistryBackup,
    restoreVrcRegistryBackupFromFile,
    saveVrcRegistryBackupToFile
};
