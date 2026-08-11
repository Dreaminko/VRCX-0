import { spawnSync } from 'node:child_process';
import path from 'node:path';

const expectedDiagnosticsByFile = new Map<string, number>([
    ['src/components/dashboard/widgets/DashboardFeedWidget.tsx', 3],
    ['src/components/data-table/DataTableView.tsx', 1],
    ['src/components/dialogs/avatar-dialog/useAvatarDialogState.ts', 1],
    ['src/components/dialogs/AvatarDialogTabbedView.tsx', 1],
    ['src/components/dialogs/AvatarOwnerEditDialogs.tsx', 1],
    ['src/components/dialogs/BoopEmojiDialog.tsx', 1],
    ['src/components/dialogs/EntityDialogScaffold.tsx', 1],
    ['src/components/dialogs/group-dialog/GroupDialogTabbedView.tsx', 5],
    ['src/components/dialogs/group-dialog/GroupModerationWorkspace.tsx', 4],
    ['src/components/dialogs/group-dialog/GroupPostEditorDialog.tsx', 1],
    ['src/components/dialogs/group-dialog/useGroupModerationTable.ts', 1],
    ['src/components/dialogs/invite-message/InviteMessagePanel.tsx', 1],
    [
        'src/components/dialogs/previous-instances-table/PreviousInstancesViewParts.test.tsx',
        1
    ],
    [
        'src/components/dialogs/user-dialog/components/UserDialogProfileMediaPanel.tsx',
        1
    ],
    ['src/components/dialogs/user-dialog/useUserActivityPanelController.ts', 2],
    ['src/components/dialogs/user-dialog/useUserDialogLocationPanel.ts', 3],
    ['src/components/dialogs/user-dialog/useUserDialogProfileResource.ts', 2],
    ['src/components/dialogs/user-dialog/useUserDialogSupplementalData.ts', 2],
    ['src/components/dialogs/user-dialog/useUserDialogTabData.ts', 6],
    ['src/components/dialogs/UserActivityPanelImpl.tsx', 1],
    ['src/components/dialogs/UserDialogTabbedView.tsx', 7],
    ['src/components/dialogs/world-dialog/useWorldDialogData.ts', 3],
    ['src/components/dialogs/world-dialog/useWorldDialogCurrentInstance.ts', 1],
    [
        'src/components/dialogs/world-dialog/WorldDialogInstanceUsers.test.tsx',
        1
    ],
    ['src/components/dialogs/world-dialog/WorldDialogTabbedView.tsx', 5],
    ['src/components/dialogs/WorldDialogContentWorkflow.tsx', 1],
    ['src/components/hosts/LaunchDialogHost.tsx', 2],
    ['src/components/hosts/system-dialogs/LaunchOptionsDialog.tsx', 1],
    ['src/components/hosts/system-dialogs/RegistryBackupDialog.tsx', 1],
    ['src/components/hosts/system-dialogs/VRChatConfigDialog.tsx', 1],
    ['src/components/hosts/tools-dialogs/AppLauncherDialog.tsx', 1],
    ['src/components/hosts/tools-dialogs/ExportListDialogs.tsx', 2],
    ['src/components/hosts/tools-dialogs/GroupCalendarDialog.tsx', 1],
    ['src/components/hosts/tools-dialogs/NoteExportDialog.tsx', 1],
    [
        'src/components/hosts/tools-dialogs/presence-automation/PresenceAutomationDialog.tsx',
        3
    ],
    ['src/components/hosts/VrcNotificationCenterHost.tsx', 1],
    ['src/components/layout/AppMenuBar.tsx', 1],
    ['src/components/location/useLocationMetadata.ts', 2],
    ['src/components/media/ImageCropDialog.tsx', 1],
    ['src/components/media/imageCropDialogSession.ts', 2],
    ['src/components/sidebar/friends-sidebar/FriendsSidebarLocation.tsx', 1],
    ['src/components/sidebar/FriendsSidebar.tsx', 6],
    ['src/components/sidebar/GroupsSidebar.tsx', 1],
    ['src/components/sidebar/quick-search/useQuickSearchResults.ts', 1],
    ['src/components/sidebar/useVirtualSidebarRows.ts', 3],
    ['src/components/user-hover-card/useUserHoverCardData.ts', 2],
    ['src/features/auth/useLoginAutoLogin.ts', 1],
    ['src/features/auth/useLoginPageState.ts', 1],
    ['src/features/dashboard/useDashboardEditorState.ts', 1],
    ['src/features/favorites/remoteEntityCacheFallbacks.ts', 3],
    ['src/features/favorites/useAvatarDetailFallbacks.ts', 5],
    ['src/features/favorites/useWorldDetailFallbacks.ts', 5],
    ['src/features/feed/columns/useFeedColumnRows.ts', 1],
    ['src/features/feed/FeedPage.tsx', 6],
    ['src/features/feed/useFeedFriendActions.ts', 2],
    ['src/features/feed/useFeedPageController.ts', 1],
    ['src/features/feed/useFeedRows.ts', 2],
    ['src/features/friends/useFriendListRowActions.ts', 1],
    ['src/features/friends/useFriendListRows.ts', 1],
    ['src/features/game-log/useGameLogPageController.ts', 1],
    ['src/features/game-log/useGameLogRows.ts', 1],
    ['src/features/moderation/components/ModerationColumns.tsx', 1],
    ['src/features/my-avatars/AvatarStylesDialog.tsx', 1],
    ['src/features/my-avatars/useMyAvatarsRows.ts', 1],
    ['src/features/notifications/useNotificationActions.ts', 5],
    ['src/features/notifications/useNotificationRuntime.ts', 1],
    ['src/features/settings/useSettingsEffects.ts', 7],
    ['src/features/tools/components/ScreenshotThumbnailGrid.tsx', 2],
    ['src/features/tools/ScreenshotMetadataPage.tsx', 1],
    ['src/features/tools/useGalleryPageController.ts', 1],
    ['src/features/tools/useInventoryPageState.ts', 1],
    ['src/features/tools/useToolsPageState.ts', 2]
]);

type Diagnostic = {
    filename: string;
};

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function readDiagnostics(value: unknown): Diagnostic[] {
    if (!isRecord(value) || !Array.isArray(value.diagnostics)) {
        throw new Error('Oxlint returned an invalid JSON payload.');
    }
    return value.diagnostics.map((diagnostic) => {
        if (!isRecord(diagnostic) || typeof diagnostic.filename !== 'string') {
            throw new Error('Oxlint returned a diagnostic without a filename.');
        }
        return { filename: diagnostic.filename.replaceAll('\\', '/') };
    });
}

function compareDiagnosticCounts(diagnostics: Diagnostic[]): string[] {
    const actualByFile = new Map<string, number>();
    for (const diagnostic of diagnostics) {
        actualByFile.set(
            diagnostic.filename,
            (actualByFile.get(diagnostic.filename) ?? 0) + 1
        );
    }

    const changedFiles = new Set([
        ...expectedDiagnosticsByFile.keys(),
        ...actualByFile.keys()
    ]);
    const failures: string[] = [];
    for (const filename of Array.from(changedFiles).sort()) {
        const expected = expectedDiagnosticsByFile.get(filename) ?? 0;
        const actual = actualByFile.get(filename) ?? 0;
        if (expected !== actual) {
            failures.push(
                `${filename}: expected ${expected}, received ${actual}`
            );
        }
    }
    return failures;
}

function run(): void {
    const oxlintPath = path.resolve('node_modules/oxlint/bin/oxlint');
    const result = spawnSync(
        process.execPath,
        [
            oxlintPath,
            'src',
            '--react-plugin',
            '-A',
            'all',
            '-D',
            'react/exhaustive-deps',
            '--format',
            'json'
        ],
        {
            cwd: process.cwd(),
            encoding: 'utf8',
            maxBuffer: 16 * 1024 * 1024
        }
    );
    if (result.error) {
        throw result.error;
    }
    if (result.status !== 0 && result.status !== 1) {
        throw new Error(result.stderr || `Oxlint exited with ${result.status}`);
    }

    const diagnostics = readDiagnostics(JSON.parse(result.stdout));
    const failures = compareDiagnosticCounts(diagnostics);
    if (failures.length) {
        throw new Error(
            `React Hook dependency debt changed:\n${failures.join('\n')}`
        );
    }
    console.log(
        `React Hook dependency debt unchanged at ${diagnostics.length} diagnostics across ${expectedDiagnosticsByFile.size} files.`
    );
}

run();
