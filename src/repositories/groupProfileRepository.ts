import type {
    GroupAuditLogRow,
    GroupInstanceRecord,
    GroupMemberRow,
    GroupProfileRecord
} from '@/domain/entities/profileEntities';
import {
    entityQueryPolicies,
    fetchCachedData,
    queryKeys
} from '@/lib/entityQueryCache';
import { commands } from '@/platform/tauri/bindings';
import { createDefaultGroupRef } from '@/shared/utils/groupTransforms';
import { replaceBioSymbols } from '@/shared/utils/string';

import { VRCHAT_API_DEFAULT_PAGE_SIZE } from './paginationConstants';
import {
    createRequestError,
    notifyVrchatAuthFailure,
    parseJsonResponse,
    unwrapErrorMessage,
    type QueryParams
} from './vrchatRequest';

type GroupRecord = Record<string, unknown>;
type GroupGalleryFileRow = GroupRecord & {
    approved?: boolean;
    approvedAt?: string | null;
    approvedByUserId?: string | null;
    createdAt?: string;
    fileId: string;
    galleryId: string;
    groupId: string;
    id: string;
    imageUrl?: string;
    submittedByUserId?: string;
};

type GroupUserGroupRow = GroupRecord & {
    bannerId?: string;
    bannerUrl?: string;
    description?: string;
    discriminator?: string;
    groupId: string;
    iconId?: string;
    iconUrl?: string;
    id: string;
    isRepresenting?: boolean;
    lastPostCreatedAt?: string | null;
    lastPostReadAt?: string | null;
    memberCount?: number;
    memberVisibility?: string;
    mutualGroup?: boolean;
    name?: string;
    ownerId?: string;
    privacy?: string;
    shortCode?: string;
};

type GroupInstancesResponse =
    | GroupInstanceRecord[]
    | (GroupRecord & {
          fetchedAt?: string;
          instances?: GroupInstanceRecord[];
      });

type GroupLogsPage = {
    hasNext: boolean;
    results: GroupAuditLogRow[];
    totalCount: number | null;
};

type GroupModerationRow = Partial<GroupMemberRow> & {
    groupId: string;
    id: string;
    userId: string;
};
type VrchatApiResult = {
    status: number;
    data: unknown;
    raw: unknown;
};

interface PageRequest {
    n: number;
    offset: number;
}

interface CollectPagesOptions {
    pageSize?: number;
    maxPages?: number;
}

interface GroupProfileInput {
    groupId?: unknown;
    endpoint?: string;
    includeRoles?: boolean;
    force?: boolean;
    dialog?: boolean;
}

interface GroupIdInput {
    groupId?: unknown;
    endpoint?: string;
}

interface GroupUserInput extends GroupIdInput {
    userId?: unknown;
}

interface GroupPostInput extends GroupIdInput {
    postId?: unknown;
    params?: Record<string, unknown>;
}

interface GroupPageInput extends GroupIdInput {
    n?: number;
    offset?: number;
}

interface GroupMembersInput extends GroupPageInput {
    sort?: string;
    roleId?: string;
    force?: boolean;
}

interface GroupMembersSearchInput extends GroupPageInput {
    query?: unknown;
}

interface GroupGalleryInput extends GroupPageInput {
    galleryId?: unknown;
    force?: boolean;
}

interface GroupJoinRequestInput extends GroupPageInput {
    blocked?: boolean;
}

interface GroupJoinRequestResponseInput extends GroupUserInput {
    action?: unknown;
    block?: boolean;
}

interface GroupLogsInput extends GroupPageInput {
    eventTypes?: unknown;
}

interface GroupRepresentationInput extends GroupIdInput {
    isRepresenting?: unknown;
}

interface GroupMemberPropsInput extends GroupUserInput {
    params?: Record<string, unknown>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object');
}

function unwrapVrchatGroupResponse<TJson = GroupRecord>(
    response: VrchatApiResult,
    path: string
) {
    const json = parseJsonResponse(response.data);
    if (response.status >= 400 || (isRecord(json) && 'error' in json)) {
        const requestError = createRequestError(
            unwrapErrorMessage(json, response.status, {
                fallbackMessage: 'VRChat group request failed'
            }),
            response.status,
            path,
            json
        );
        notifyVrchatAuthFailure(requestError);
        throw requestError;
    }

    return {
        json: json as TJson,
        status: response.status,
        raw: response.raw
    };
}

function normalizeEntityId(value: unknown): string {
    const normalize = (text: string) => {
        const normalized = text.trim();
        return normalized === '[object Object]' ? '' : normalized;
    };
    if (typeof value === 'string') {
        return normalize(value);
    }
    if (typeof value === 'number' || typeof value === 'bigint') {
        return normalize(String(value));
    }
    return '';
}

function normalizeString(value: unknown): string {
    return typeof value === 'string' ? value.trim() : '';
}

function normalizeText(value: unknown): string {
    if (typeof value !== 'string' || !value) {
        return '';
    }
    const rawText = value.trim();
    if (rawText === '[object Object]') {
        return '';
    }
    return replaceBioSymbols(rawText).trim();
}

function normalizeArray(values: unknown): string[] {
    if (!Array.isArray(values)) {
        return [];
    }

    return values
        .map((value) =>
            typeof value === 'string'
                ? value.trim()
                : String(value ?? '').trim()
        )
        .filter(Boolean);
}

function parseInteger(value: unknown): number {
    const parsed = Number.parseInt(String(value), 10);
    return Number.isFinite(parsed) ? parsed : 0;
}

function parseOptionalInteger(value: unknown): number | null {
    const parsed = Number.parseInt(String(value), 10);
    return Number.isFinite(parsed) ? parsed : null;
}

function normalizeGroupRoles(values: unknown): GroupRecord[] {
    if (!Array.isArray(values)) {
        return [];
    }

    return values
        .filter((role): role is GroupRecord =>
            Boolean(role && typeof role === 'object')
        )
        .map((role) => ({
            ...role,
            id: normalizeEntityId(role.id),
            name: normalizeText(role.name),
            description: normalizeText(role.description),
            permissions: normalizeArray(role.permissions)
        }));
}

function normalizeGroupProfile(
    group: GroupRecord | null | undefined
): GroupProfileRecord {
    const base = createDefaultGroupRef(group ?? {});
    const owner = isRecord(base.owner) ? base.owner : {};
    const shortCode = normalizeString(base.shortCode);
    const discriminator = normalizeString(base.discriminator);
    const ownerId =
        normalizeEntityId(base.ownerId) ||
        normalizeEntityId(owner.id) ||
        normalizeEntityId(owner.userId) ||
        normalizeEntityId(owner.user_id);
    const ownerDisplayName =
        normalizeText(base.ownerDisplayName) ||
        normalizeText(base.ownerName) ||
        normalizeText(owner.displayName) ||
        normalizeText(owner.username) ||
        normalizeText(owner.name);
    const groupUrl =
        shortCode && discriminator
            ? `https://vrc.group/${shortCode}.${discriminator}`
            : '';

    return {
        ...base,
        id: normalizeEntityId(base.id || base.groupId),
        name: normalizeText(base.name),
        displayName: normalizeText(base.displayName || base.name),
        description: normalizeText(base.description),
        rules: normalizeText(base.rules),
        shortCode,
        discriminator,
        bannerUrl: normalizeString(base.bannerUrl),
        iconUrl: normalizeString(base.iconUrl),
        createdAt: typeof base.createdAt === 'string' ? base.createdAt : '',
        updatedAt: typeof base.updatedAt === 'string' ? base.updatedAt : '',
        memberCount: parseInteger(base.memberCount),
        onlineMemberCount: parseInteger(base.onlineMemberCount),
        ownerId,
        ownerDisplayName,
        privacy: normalizeString(base.privacy),
        membershipStatus: normalizeString(base.membershipStatus),
        memberCountSyncedAt:
            typeof base.memberCountSyncedAt === 'string'
                ? base.memberCountSyncedAt
                : '',
        languages: normalizeArray(base.languages),
        links: normalizeArray(base.links),
        tags: normalizeArray(base.tags),
        roles: normalizeGroupRoles(base.roles),
        url: groupUrl
    };
}

function responseRows<TRow = unknown>(json: unknown, key = ''): TRow[] {
    if (Array.isArray(json)) {
        return json as TRow[];
    }

    if (key && isRecord(json) && Array.isArray(json[key])) {
        return json[key] as TRow[];
    }

    return [];
}

function responsePage<TRow = unknown>(json: unknown, key = '') {
    const results = responseRows<TRow>(json, key);
    const record = isRecord(json) ? json : {};
    return {
        hasNext: record.hasNext === true,
        results,
        totalCount: parseOptionalInteger(record.totalCount)
    };
}

async function collectPages<TRow = unknown>(
    fetchPage: (page: PageRequest) => Promise<TRow[]>,
    {
        pageSize = VRCHAT_API_DEFAULT_PAGE_SIZE,
        maxPages = Number.POSITIVE_INFINITY
    }: CollectPagesOptions = {}
) {
    const rows: TRow[] = [];

    for (let page = 0; page < maxPages; page += 1) {
        const nextRows = await fetchPage({
            n: pageSize,
            offset: page * pageSize
        });
        rows.push(...nextRows);

        if (nextRows.length < pageSize) {
            break;
        }
    }

    return rows;
}

function normalize(group: GroupRecord): GroupProfileRecord {
    return normalizeGroupProfile(group);
}

async function getGroupProfile({
    groupId,
    endpoint = '',
    includeRoles = true,
    force = false,
    dialog = false
}: GroupProfileInput): Promise<GroupProfileRecord> {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.getGroupProfile requires a group id.'
        );
    }

    const json = await fetchCachedData({
        queryKey: queryKeys.group(normalizedGroupId, includeRoles, endpoint),
        policy: dialog
            ? entityQueryPolicies.groupDialog
            : entityQueryPolicies.group,
        force,
        queryFn: () =>
            fetchGroupProfile({
                groupId: normalizedGroupId,
                endpoint,
                includeRoles
            })
    });

    return json;
}

async function fetchGroupProfile({
    groupId,
    endpoint = '',
    includeRoles = true
}: GroupProfileInput): Promise<GroupProfileRecord> {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.fetchGroupProfile requires a group id.'
        );
    }

    const response = unwrapVrchatGroupResponse<GroupRecord>(
        await commands.appVrchatGroupGet({
            groupId: normalizedGroupId,
            includeRoles: Boolean(includeRoles),
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}`
    );
    return normalize(isRecord(response.json) ? response.json : {});
}

async function getUserGroups({
    userId,
    endpoint = ''
}: Pick<GroupUserInput, 'userId' | 'endpoint'>) {
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.getUserGroups requires a user id.'
        );
    }

    const rows = await fetchCachedData<GroupUserGroupRow[]>({
        queryKey: queryKeys.userGroups(normalizedUserId, endpoint),
        policy: entityQueryPolicies.groupCollection,
        queryFn: async () => {
            const response = unwrapVrchatGroupResponse<GroupUserGroupRow[]>(
                await commands.appVrchatGroupUserGroupsGet({
                    userId: normalizedUserId,
                    endpoint
                }),
                `users/${encodeURIComponent(normalizedUserId)}/groups`
            );
            return Array.isArray(response.json) ? response.json : [];
        }
    });
    return rows.map((group) => normalize(isRecord(group) ? group : {}));
}

async function getGroupPosts({
    groupId,
    endpoint = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0
}: GroupPageInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.getGroupPosts requires a group id.'
        );
    }

    const response = unwrapVrchatGroupResponse(
        await commands.appVrchatGroupPostsGet({
            groupId: normalizedGroupId,
            n,
            offset,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/posts`
    );
    return responseRows<GroupRecord>(response.json, 'posts');
}

async function getAllGroupPosts({ groupId, endpoint = '' }: GroupIdInput) {
    return collectPages(({ n, offset }) =>
        getGroupPosts({ groupId, endpoint, n, offset })
    );
}

async function createGroupPost({
    groupId,
    params = {},
    endpoint = ''
}: Pick<GroupPostInput, 'groupId' | 'params' | 'endpoint'>) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.createGroupPost requires a group id.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupPostCreate({
            groupId: normalizedGroupId,
            params,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/posts`
    );
}

async function editGroupPost({
    groupId,
    postId,
    params = {},
    endpoint = ''
}: GroupPostInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedPostId = normalizeEntityId(postId);
    if (!normalizedGroupId || !normalizedPostId) {
        throw new Error(
            'GroupProfileRepository.editGroupPost requires group and post ids.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupPostEdit({
            groupId: normalizedGroupId,
            postId: normalizedPostId,
            params,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/posts/${encodeURIComponent(normalizedPostId)}`
    );
}

async function deleteGroupPost({
    groupId,
    postId,
    endpoint = ''
}: GroupPostInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedPostId = normalizeEntityId(postId);
    if (!normalizedGroupId || !normalizedPostId) {
        throw new Error(
            'GroupProfileRepository.deleteGroupPost requires group and post ids.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupPostDelete({
            groupId: normalizedGroupId,
            postId: normalizedPostId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/posts/${encodeURIComponent(normalizedPostId)}`
    );
}

async function getGroupMembers({
    groupId,
    endpoint = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0,
    sort = 'joinedAt:desc',
    roleId = '',
    force = false
}: GroupMembersInput): Promise<GroupMemberRow[]> {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.getGroupMembers requires a group id.'
        );
    }

    const params: QueryParams = { n, offset, sort };
    if (roleId) {
        params.roleId = roleId;
    }

    return fetchCachedData({
        queryKey: queryKeys.groupMembers(
            { groupId: normalizedGroupId, ...params },
            endpoint
        ),
        policy: entityQueryPolicies.groupCollection,
        force,
        queryFn: async () => {
            const response = unwrapVrchatGroupResponse(
                await commands.appVrchatGroupMembersGet({
                    groupId: normalizedGroupId,
                    n,
                    offset,
                    sort,
                    roleId,
                    endpoint
                }),
                `groups/${encodeURIComponent(normalizedGroupId)}/members`
            );
            return responseRows<GroupMemberRow>(response.json, 'members');
        }
    });
}

async function getGroupMembersSearch({
    groupId,
    query = '',
    endpoint = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0
}: GroupMembersSearchInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedQuery = normalizeText(query);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.getGroupMembersSearch requires a group id.'
        );
    }

    const response = unwrapVrchatGroupResponse(
        await commands.appVrchatGroupMembersSearch({
            groupId: normalizedGroupId,
            n,
            offset,
            query: normalizedQuery,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/members/search`
    );
    return responseRows<GroupMemberRow>(response.json, 'results');
}

async function getAllGroupMembers({
    groupId,
    endpoint = '',
    sort = 'joinedAt:desc',
    roleId = '',
    force = false
}: Omit<GroupMembersInput, 'n' | 'offset'>) {
    return collectPages(({ n, offset }) =>
        getGroupMembers({ groupId, endpoint, n, offset, sort, roleId, force })
    );
}

async function getGroupGallery({
    groupId,
    galleryId,
    endpoint = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0,
    force = false
}: GroupGalleryInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedGalleryId = normalizeEntityId(galleryId);
    if (!normalizedGroupId || !normalizedGalleryId) {
        throw new Error(
            'GroupProfileRepository.getGroupGallery requires group and gallery ids.'
        );
    }

    const params: QueryParams = { n, offset };
    return fetchCachedData({
        queryKey: queryKeys.groupGallery(
            {
                groupId: normalizedGroupId,
                galleryId: normalizedGalleryId,
                ...params
            },
            endpoint
        ),
        policy: entityQueryPolicies.groupCollection,
        force,
        queryFn: async () => {
            const response = unwrapVrchatGroupResponse(
                await commands.appVrchatGroupGalleryGet({
                    groupId: normalizedGroupId,
                    galleryId: normalizedGalleryId,
                    n,
                    offset,
                    endpoint
                }),
                `groups/${encodeURIComponent(normalizedGroupId)}/galleries/${encodeURIComponent(normalizedGalleryId)}`
            );
            return responseRows<GroupGalleryFileRow>(response.json, 'files');
        }
    });
}

async function getAllGroupGallery({
    groupId,
    galleryId,
    endpoint = '',
    force = false
}: Omit<GroupGalleryInput, 'n' | 'offset'>) {
    return collectPages(({ n, offset }) =>
        getGroupGallery({ groupId, galleryId, endpoint, n, offset, force })
    );
}

async function joinGroup({ groupId, endpoint = '' }: GroupIdInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.joinGroup requires a group id.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupJoin({
            groupId: normalizedGroupId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/join`
    );
}

async function leaveGroup({ groupId, endpoint = '' }: GroupIdInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.leaveGroup requires a group id.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupLeave({
            groupId: normalizedGroupId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/leave`
    );
}

async function cancelGroupRequest({ groupId, endpoint = '' }: GroupIdInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.cancelGroupRequest requires a group id.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupRequestCancel({
            groupId: normalizedGroupId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/requests`
    );
}

async function sendGroupInvite({
    groupId,
    userId,
    endpoint = ''
}: GroupUserInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedGroupId || !normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.sendGroupInvite requires group and user ids.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupInviteSend({
            groupId: normalizedGroupId,
            userId: normalizedUserId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/invites`
    );
}

async function kickGroupMember({
    groupId,
    userId,
    endpoint = ''
}: GroupUserInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedGroupId || !normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.kickGroupMember requires group and user ids.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupMemberKick({
            groupId: normalizedGroupId,
            userId: normalizedUserId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/members/${encodeURIComponent(normalizedUserId)}`
    );
}

async function banGroupMember({
    groupId,
    userId,
    endpoint = ''
}: GroupUserInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedGroupId || !normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.banGroupMember requires group and user ids.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupMemberBan({
            groupId: normalizedGroupId,
            userId: normalizedUserId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/bans`
    );
}

async function unbanGroupMember({
    groupId,
    userId,
    endpoint = ''
}: GroupUserInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedGroupId || !normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.unbanGroupMember requires group and user ids.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupMemberUnban({
            groupId: normalizedGroupId,
            userId: normalizedUserId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/bans/${encodeURIComponent(normalizedUserId)}`
    );
}

async function deleteSentGroupInvite({
    groupId,
    userId,
    endpoint = ''
}: GroupUserInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedGroupId || !normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.deleteSentGroupInvite requires group and user ids.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupInviteDelete({
            groupId: normalizedGroupId,
            userId: normalizedUserId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/invites/${encodeURIComponent(normalizedUserId)}`
    );
}

async function respondGroupJoinRequest({
    groupId,
    userId,
    action,
    block = false,
    endpoint = ''
}: GroupJoinRequestResponseInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedUserId = normalizeEntityId(userId);
    const normalizedAction = normalizeString(action);
    if (!normalizedGroupId || !normalizedUserId || !normalizedAction) {
        throw new Error(
            'GroupProfileRepository.respondGroupJoinRequest requires group id, user id, and action.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupJoinRequestRespond({
            groupId: normalizedGroupId,
            userId: normalizedUserId,
            action: normalizedAction,
            block: Boolean(block),
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/requests/${encodeURIComponent(normalizedUserId)}`
    );
}

async function deleteBlockedGroupRequest({
    groupId,
    userId,
    endpoint = ''
}: GroupUserInput) {
    return kickGroupMember({ groupId, userId, endpoint });
}

async function getGroupInstances({
    groupId,
    userId,
    endpoint = ''
}: GroupUserInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedGroupId || !normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.getGroupInstances requires group and user ids.'
        );
    }

    const response = unwrapVrchatGroupResponse<GroupInstancesResponse>(
        await commands.appVrchatGroupInstancesGet({
            groupId: normalizedGroupId,
            userId: normalizedUserId,
            endpoint
        }),
        `users/${encodeURIComponent(normalizedUserId)}/instances/groups/${encodeURIComponent(normalizedGroupId)}`
    );
    return response;
}

async function getGroupBans({
    groupId,
    endpoint = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0
}: GroupPageInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.getGroupBans requires a group id.'
        );
    }

    const response = unwrapVrchatGroupResponse(
        await commands.appVrchatGroupBansGet({
            groupId: normalizedGroupId,
            n,
            offset,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/bans`
    );
    return responseRows<GroupModerationRow>(response.json, 'bans');
}

async function getAllGroupBans({ groupId, endpoint = '' }: GroupIdInput) {
    return collectPages(({ n, offset }) =>
        getGroupBans({ groupId, endpoint, n, offset })
    );
}

async function getGroupInvites({
    groupId,
    endpoint = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0
}: GroupPageInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.getGroupInvites requires a group id.'
        );
    }

    const response = unwrapVrchatGroupResponse(
        await commands.appVrchatGroupInvitesGet({
            groupId: normalizedGroupId,
            n,
            offset,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/invites`
    );
    return responseRows<GroupModerationRow>(response.json, 'invites');
}

async function getAllGroupInvites({ groupId, endpoint = '' }: GroupIdInput) {
    return collectPages(({ n, offset }) =>
        getGroupInvites({ groupId, endpoint, n, offset })
    );
}

async function getGroupJoinRequests({
    groupId,
    endpoint = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0,
    blocked = false
}: GroupJoinRequestInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.getGroupJoinRequests requires a group id.'
        );
    }

    const response = unwrapVrchatGroupResponse(
        await commands.appVrchatGroupJoinRequestsGet({
            groupId: normalizedGroupId,
            n,
            offset,
            blocked: Boolean(blocked),
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/requests`
    );
    return responseRows<GroupModerationRow>(response.json, 'requests');
}

async function getAllGroupJoinRequests({
    groupId,
    endpoint = '',
    blocked = false
}: Omit<GroupJoinRequestInput, 'n' | 'offset'>) {
    return collectPages(({ n, offset }) =>
        getGroupJoinRequests({ groupId, endpoint, n, offset, blocked })
    );
}

async function getGroupAuditLogTypes({ groupId, endpoint = '' }: GroupIdInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.getGroupAuditLogTypes requires a group id.'
        );
    }

    const response = unwrapVrchatGroupResponse<unknown[]>(
        await commands.appVrchatGroupAuditLogTypesGet({
            groupId: normalizedGroupId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/auditLogTypes`
    );
    return Array.isArray(response.json) ? response.json : [];
}

async function getGroupLogs({
    groupId,
    endpoint = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0,
    eventTypes = []
}: GroupLogsInput) {
    const page = await getGroupLogsPage({
        groupId,
        endpoint,
        n,
        offset,
        eventTypes
    });
    return page.results;
}

async function getGroupLogsPage({
    groupId,
    endpoint = '',
    n = VRCHAT_API_DEFAULT_PAGE_SIZE,
    offset = 0,
    eventTypes = []
}: GroupLogsInput): Promise<GroupLogsPage> {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.getGroupLogs requires a group id.'
        );
    }

    const eventTypesValue =
        Array.isArray(eventTypes) && eventTypes.length
            ? eventTypes.join(',')
            : '';

    const response = unwrapVrchatGroupResponse(
        await commands.appVrchatGroupLogsGet({
            groupId: normalizedGroupId,
            n,
            offset,
            eventTypes: eventTypesValue,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/auditLogs`
    );
    return responsePage<GroupAuditLogRow>(response.json, 'results');
}

async function getAllGroupLogs({
    groupId,
    endpoint = '',
    eventTypes = []
}: Omit<GroupLogsInput, 'n' | 'offset'>) {
    const rows: GroupAuditLogRow[] = [];
    const seenIds = new Set<string>();
    const pageSize = VRCHAT_API_DEFAULT_PAGE_SIZE;
    const maxPages = 50;

    for (let page = 0; page < maxPages; page += 1) {
        const nextPage = await getGroupLogsPage({
            groupId,
            endpoint,
            n: pageSize,
            offset: page * pageSize,
            eventTypes
        });

        for (const row of nextPage.results) {
            const id = normalizeEntityId(row.id);
            if (id) {
                if (seenIds.has(id)) {
                    continue;
                }
                seenIds.add(id);
            }
            rows.push(row);
        }

        if (!nextPage.hasNext) {
            break;
        }
    }

    return rows;
}

async function setGroupRepresentation({
    groupId,
    isRepresenting,
    endpoint = ''
}: GroupRepresentationInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.setGroupRepresentation requires a group id.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupRepresentationSet({
            groupId: normalizedGroupId,
            isRepresenting: Boolean(isRepresenting),
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/representation`
    );
}

async function setGroupMemberProps({
    groupId,
    userId,
    params = {},
    endpoint = ''
}: GroupMemberPropsInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedGroupId || !normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.setGroupMemberProps requires group and user ids.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupMemberPropsSet({
            groupId: normalizedGroupId,
            userId: normalizedUserId,
            params,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/members/${encodeURIComponent(normalizedUserId)}`
    );
}

async function blockGroup({ groupId, endpoint = '' }: GroupIdInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    if (!normalizedGroupId) {
        throw new Error(
            'GroupProfileRepository.blockGroup requires a group id.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupBlock({
            groupId: normalizedGroupId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/block`
    );
}

async function unblockGroup({
    groupId,
    userId,
    endpoint = ''
}: GroupUserInput) {
    const normalizedGroupId = normalizeEntityId(groupId);
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedGroupId || !normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.unblockGroup requires group and user ids.'
        );
    }

    return unwrapVrchatGroupResponse(
        await commands.appVrchatGroupUnblock({
            groupId: normalizedGroupId,
            userId: normalizedUserId,
            endpoint
        }),
        `groups/${encodeURIComponent(normalizedGroupId)}/bans/${encodeURIComponent(normalizedUserId)}`
    );
}

async function getUsersGroupInstances({
    userId,
    endpoint = ''
}: Pick<GroupUserInput, 'userId' | 'endpoint'>) {
    const normalizedUserId = normalizeEntityId(userId);
    if (!normalizedUserId) {
        throw new Error(
            'GroupProfileRepository.getUsersGroupInstances requires a user id.'
        );
    }

    const response = unwrapVrchatGroupResponse<GroupInstancesResponse>(
        await commands.appVrchatGroupUserInstancesGet({
            userId: normalizedUserId,
            endpoint
        }),
        `users/${encodeURIComponent(normalizedUserId)}/instances/groups`
    );
    return response;
}

const groupProfileRepository = Object.freeze({
    normalize,
    fetchGroupProfile,
    getGroupProfile,
    getUserGroups,
    getGroupPosts,
    getAllGroupPosts,
    createGroupPost,
    editGroupPost,
    deleteGroupPost,
    getGroupMembers,
    getGroupMembersSearch,
    getAllGroupMembers,
    getGroupGallery,
    getAllGroupGallery,
    joinGroup,
    leaveGroup,
    cancelGroupRequest,
    sendGroupInvite,
    kickGroupMember,
    banGroupMember,
    unbanGroupMember,
    deleteSentGroupInvite,
    respondGroupJoinRequest,
    deleteBlockedGroupRequest,
    getGroupInstances,
    getGroupBans,
    getAllGroupBans,
    getGroupInvites,
    getAllGroupInvites,
    getGroupJoinRequests,
    getAllGroupJoinRequests,
    getGroupAuditLogTypes,
    getGroupLogs,
    getGroupLogsPage,
    getAllGroupLogs,
    setGroupRepresentation,
    setGroupMemberProps,
    blockGroup,
    unblockGroup,
    getUsersGroupInstances
});

export {
    normalize,
    getGroupProfile,
    getUserGroups,
    getGroupPosts,
    getAllGroupPosts,
    createGroupPost,
    editGroupPost,
    deleteGroupPost,
    getGroupMembers,
    getGroupMembersSearch,
    getAllGroupMembers,
    getGroupGallery,
    getAllGroupGallery,
    joinGroup,
    leaveGroup,
    cancelGroupRequest,
    sendGroupInvite,
    kickGroupMember,
    banGroupMember,
    unbanGroupMember,
    deleteSentGroupInvite,
    respondGroupJoinRequest,
    deleteBlockedGroupRequest,
    getGroupInstances,
    getGroupBans,
    getAllGroupBans,
    getGroupInvites,
    getAllGroupInvites,
    getGroupJoinRequests,
    getAllGroupJoinRequests,
    getGroupAuditLogTypes,
    getGroupLogs,
    getGroupLogsPage,
    getAllGroupLogs,
    setGroupRepresentation,
    setGroupMemberProps,
    blockGroup,
    unblockGroup,
    getUsersGroupInstances
};
export type {
    GroupAuditLogData,
    GroupAuditLogRow,
    GroupInstanceRecord,
    GroupMemberRow,
    GroupMemberUser,
    GroupProfileRecord
} from '@/domain/entities/profileEntities';
export default groupProfileRepository;
