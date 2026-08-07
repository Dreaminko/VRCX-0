import { describe, expect, it } from 'vitest';

import { formatCsvField, formatCsvRow, needsCsvQuotes } from './csv';

describe('formatCsvField', () => {
    it('leaves a plain field unquoted so exported CSVs stay readable', () => {
        expect(formatCsvField('usr_123')).toBe('usr_123');
    });

    it('quotes and escapes a display name containing a comma, so it is not split into extra columns when opened in a spreadsheet', () => {
        expect(formatCsvField('Doe, Jane')).toBe('"Doe, Jane"');
    });

    it('doubles up embedded quotes so a name containing a quote mark round-trips through Excel/Sheets correctly', () => {
        expect(formatCsvField('Say "hi"')).toBe('"Say ""hi"""');
    });

    it('quotes a field containing a raw newline or other control character, so one exported row cannot corrupt the CSV structure', () => {
        expect(formatCsvField('line one\nline two')).toBe(
            '"line one\nline two"'
        );
    });

    it('exports a missing value as an empty field rather than the literal text "null" or "undefined"', () => {
        expect(formatCsvField(null)).toBe('');
        expect(formatCsvField(undefined)).toBe('');
    });
});

describe('needsCsvQuotes', () => {
    it('flags commas, quotes, and control characters as requiring quoting', () => {
        expect(needsCsvQuotes('a,b')).toBe(true);
        expect(needsCsvQuotes('a"b')).toBe(true);
        expect(needsCsvQuotes('a\tb')).toBe(true);
    });

    it('does not require quoting for ordinary text', () => {
        expect(needsCsvQuotes('plain text')).toBe(false);
    });
});

describe('formatCsvRow', () => {
    it('renders a moderation/favorites export row in a fixed column order, joined by commas', () => {
        expect(
            formatCsvRow({ id: 'usr_1', displayName: 'Doe, Jane', note: '' }, [
                'id',
                'displayName',
                'note'
            ])
        ).toBe('usr_1,"Doe, Jane",');
    });

    it('emits empty fields for columns missing on the record, keeping every exported row the same width', () => {
        expect(formatCsvRow({ id: 'usr_1' }, ['id', 'displayName'])).toBe(
            'usr_1,'
        );
        expect(formatCsvRow(null, ['id', 'displayName'])).toBe(',');
    });
});
