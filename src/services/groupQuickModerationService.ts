import { commands } from '@/platform/tauri/bindings';
import type {
    GroupQuickModerationAction,
    GroupQuickModerationActionOutput,
    GroupQuickModerationOutput
} from '@/platform/tauri/bindings';

export type { GroupQuickModerationAction };

interface GroupQuickModerationInput {
    currentUserId: string;
    targetUserId: string;
    endpoint?: string;
}

interface GroupQuickModerationActionInput extends GroupQuickModerationInput {
    groupId: string;
    action: GroupQuickModerationAction;
}

export async function getGroupQuickModeration(
    input: GroupQuickModerationInput
): Promise<GroupQuickModerationOutput> {
    return commands.appUserGroupQuickModerationGet(input);
}

export async function runGroupQuickModerationAction(
    input: GroupQuickModerationActionInput
): Promise<GroupQuickModerationActionOutput> {
    return commands.appUserGroupQuickModerationAction(input);
}
