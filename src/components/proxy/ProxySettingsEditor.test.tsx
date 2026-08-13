// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import type { ComponentProps } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: (key: string) =>
            ({
                'prompt.proxy_settings.enabled': 'Enable proxy',
                'prompt.proxy_settings.enabled_description':
                    'Route requests through a proxy',
                'prompt.proxy_settings.address': 'Proxy address',
                'prompt.proxy_settings.address_description': 'Host and port',
                'prompt.proxy_settings.test': 'Test',
                'prompt.proxy_settings.restart': 'Save and restart',
                'common.actions.save': 'Save'
            })[key] ?? key
    })
}));

import { ProxySettingsEditor } from './ProxySettingsEditor';

type EditorProps = ComponentProps<typeof ProxySettingsEditor>;

function createProps(overrides: Partial<EditorProps> = {}): EditorProps {
    return {
        enabled: false,
        idPrefix: 'test',
        server: '127.0.0.1:7890',
        onEnabledChange: vi.fn(),
        onSave: vi.fn(),
        onSaveAndRestart: vi.fn(),
        onServerChange: vi.fn(),
        onTest: vi.fn(),
        ...overrides
    };
}

function expectEditorLocked() {
    expect(screen.getByRole('switch').hasAttribute('data-disabled')).toBe(true);
    expect(
        (screen.getByLabelText('Proxy address') as HTMLInputElement).disabled
    ).toBe(true);
    for (const name of ['Test', 'Save', 'Save and restart']) {
        const button = screen.getByText(name).closest('button');
        expect(button).toBeInstanceOf(HTMLButtonElement);
        expect((button as HTMLButtonElement).disabled).toBe(true);
    }
}

describe('ProxySettingsEditor', () => {
    afterEach(cleanup);

    it('preserves editable proxy state and separate save actions while idle', () => {
        const props = createProps();
        render(<ProxySettingsEditor {...props} />);

        const address = screen.getByLabelText(
            'Proxy address'
        ) as HTMLInputElement;
        expect(address.id).toBe('test-proxy-server');
        expect(address.disabled).toBe(false);

        fireEvent.click(screen.getByRole('switch'));
        fireEvent.change(address, {
            target: { value: 'proxy.example.test:8080' }
        });
        fireEvent.click(screen.getByRole('button', { name: 'Test' }));
        fireEvent.click(screen.getByRole('button', { name: 'Save' }));
        fireEvent.click(
            screen.getByRole('button', { name: 'Save and restart' })
        );

        expect(props.onEnabledChange).toHaveBeenCalled();
        expect(vi.mocked(props.onEnabledChange).mock.calls[0]?.[0]).toBe(true);
        expect(props.onServerChange).toHaveBeenCalledWith(
            'proxy.example.test:8080'
        );
        expect(props.onTest).toHaveBeenCalledOnce();
        expect(props.onSave).toHaveBeenCalledOnce();
        expect(props.onSaveAndRestart).toHaveBeenCalledOnce();
    });

    it.each([
        ['disabled', { disabled: true }],
        ['saving', { saving: true }],
        ['testing', { testing: true }]
    ] satisfies ReadonlyArray<readonly [string, Partial<EditorProps>]>)(
        'locks every proxy control while %s',
        (_label, overrides) => {
            render(<ProxySettingsEditor {...createProps(overrides)} />);
            expectEditorLocked();
        }
    );
});
