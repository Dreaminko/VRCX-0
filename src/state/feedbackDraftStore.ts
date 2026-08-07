import { create } from 'zustand';

export const FEEDBACK_SUBMIT_COOLDOWN_MS = 60_000;

type FeedbackDraftStore = {
    draft: string;
    submitting: boolean;
    thanksVisible: boolean;
    submittedAt: number | null;
    setDraft(draft: string): void;
    setSubmitting(submitting: boolean): void;
    markSubmitted(submittedAt: number): void;
};

export const useFeedbackDraftStore = create<FeedbackDraftStore>((set) => ({
    draft: '',
    submitting: false,
    thanksVisible: false,
    submittedAt: null,
    setDraft(draft: string) {
        set({ draft, thanksVisible: false });
    },
    setSubmitting(submitting: boolean) {
        set({ submitting });
    },
    markSubmitted(submittedAt: number) {
        set({ draft: '', thanksVisible: true, submittedAt });
    }
}));

export function feedbackCooldownRemainingMs(
    submittedAt: number | null,
    now: number
): number {
    if (submittedAt === null) {
        return 0;
    }
    return Math.max(0, submittedAt + FEEDBACK_SUBMIT_COOLDOWN_MS - now);
}
