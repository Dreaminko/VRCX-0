import genericWebhookFields from '../../../webhook-generic-fields.json';

export const GENERIC_WEBHOOK_FIELDS = Object.freeze(
    genericWebhookFields.map((field) => String(field))
);

export const DEFAULT_GENERIC_WEBHOOK_FIELDS = JSON.stringify(
    GENERIC_WEBHOOK_FIELDS
);
