import type { GroupProfileRecord } from '@/domain/entities/profileEntities';
import { useVrchatConfigStore } from '@/state/vrchatConfigStore';

import { normalizeLanguageOptionsFromConfig } from '../user-dialog/userProfileFields';
import { normalizeGroupLanguages } from './GroupDialogViewParts';

export function useGroupDialogLanguageRows({
    group
}: {
    group: GroupProfileRecord;
}) {
    const vrchatConfigConstants = useVrchatConfigStore(
        (state) => state.snapshot?.constants ?? null
    );

    const languageOptions = normalizeLanguageOptionsFromConfig({
        constants: vrchatConfigConstants
    });
    const languageOptionsMap = new Map(
        languageOptions.map((option) => [option.key, option])
    );
    return normalizeGroupLanguages(group, languageOptionsMap);
}
