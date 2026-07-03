import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { copyTextToClipboard } from '@/services/entityMediaService';

export function useAvatarDialogClipboard() {
    const { t } = useTranslation();

    return function copyAvatarText(text: string, label: string) {
        return copyTextToClipboard(text, {
            onCopied: () =>
                toast.success(
                    t('dialog.avatar.dynamic.value_copied', {
                        value: label
                    })
                )
        });
    };
}
