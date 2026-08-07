// @vitest-environment jsdom

import {
    cleanup,
    fireEvent,
    render,
    screen,
    waitFor
} from '@testing-library/react';
import type { ComponentProps, PropsWithChildren, ReactNode } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    t: (key: string) => key,
    endpoints: [] as {
        id: string;
        name: string;
        baseUrl: string;
        hasKey: boolean;
        models: string[];
        modelReasoning: {
            modelId: string;
            supportedEfforts: string[];
            mandatory: boolean;
        }[];
        lastDetectedAt: string | null;
    }[],
    load: vi.fn(),
    upsert: vi.fn(),
    deleteEndpoint: vi.fn(),
    detectModels: vi.fn()
}));

vi.mock('react-i18next', () => ({
    useTranslation: () => ({
        t: mocks.t
    })
}));

vi.mock('sonner', () => ({
    toast: {
        error: vi.fn(),
        success: vi.fn(),
        warning: vi.fn()
    }
}));

vi.mock('@/state/llmEndpointsStore', () => ({
    mergeModels: (...lists: string[][]) => [...new Set(lists.flat())],
    useLlmEndpointsStore: (
        selector: (state: {
            endpoints: typeof mocks.endpoints;
            loading: boolean;
            load: typeof mocks.load;
            upsert: typeof mocks.upsert;
            deleteEndpoint: typeof mocks.deleteEndpoint;
            detectModels: typeof mocks.detectModels;
        }) => unknown
    ) =>
        selector({
            endpoints: mocks.endpoints,
            loading: false,
            load: mocks.load,
            upsert: mocks.upsert,
            deleteEndpoint: mocks.deleteEndpoint,
            detectModels: mocks.detectModels
        })
}));

vi.mock('@/ui/shadcn/badge', () => ({
    Badge: ({ children }: PropsWithChildren) => <span>{children}</span>
}));

vi.mock('@/ui/shadcn/button', () => ({
    Button: ({
        children,
        variant: _variant,
        size: _size,
        ...props
    }: PropsWithChildren<
        ComponentProps<'button'> & { variant?: string; size?: string }
    >) => <button {...props}>{children}</button>
}));

vi.mock('@/ui/shadcn/combobox', () => ({
    Combobox: ({ children }: PropsWithChildren) => <div>{children}</div>,
    ComboboxChip: ({ children }: PropsWithChildren) => <span>{children}</span>,
    ComboboxChips: ({ children }: PropsWithChildren) => <div>{children}</div>,
    ComboboxChipsInput: (props: ComponentProps<'input'>) => (
        <input {...props} />
    ),
    ComboboxContent: ({ children }: PropsWithChildren) => <div>{children}</div>,
    ComboboxEmpty: ({ children }: PropsWithChildren) => <div>{children}</div>,
    ComboboxItem: ({ children }: PropsWithChildren) => <div>{children}</div>,
    ComboboxList: () => <div />,
    ComboboxValue: ({
        children
    }: {
        children: (models: string[]) => ReactNode;
    }) => <>{children([])}</>,
    useComboboxAnchor: () => null
}));

vi.mock('@/ui/shadcn/dialog', () => ({
    Dialog: ({ children, open }: PropsWithChildren<{ open: boolean }>) =>
        open ? <div>{children}</div> : null,
    DialogContent: ({ children }: PropsWithChildren) => (
        <section>{children}</section>
    ),
    DialogDescription: ({ children }: PropsWithChildren) => <p>{children}</p>,
    DialogFooter: ({ children }: PropsWithChildren) => (
        <footer>{children}</footer>
    ),
    DialogHeader: ({ children }: PropsWithChildren) => (
        <header>{children}</header>
    ),
    DialogTitle: ({ children }: PropsWithChildren) => <h1>{children}</h1>
}));

vi.mock('@/ui/shadcn/input', () => ({
    Input: (props: ComponentProps<'input'>) => <input {...props} />
}));

vi.mock('@/ui/shadcn/label', () => ({
    Label: ({ children, ...props }: ComponentProps<'label'>) => (
        <label {...props}>{children}</label>
    )
}));

vi.mock('@/ui/shadcn/select', () => ({
    Select: ({ children }: PropsWithChildren) => <div>{children}</div>,
    SelectContent: ({ children }: PropsWithChildren) => <div>{children}</div>,
    SelectGroup: ({ children }: PropsWithChildren) => <div>{children}</div>,
    SelectItem: ({ children }: PropsWithChildren) => <div>{children}</div>,
    SelectTrigger: ({ children }: PropsWithChildren) => <div>{children}</div>,
    SelectValue: () => null
}));

vi.mock('@/ui/shadcn/tooltip', () => ({
    Tooltip: ({ children }: PropsWithChildren) => <>{children}</>,
    TooltipContent: ({ children }: PropsWithChildren) => <>{children}</>,
    TooltipTrigger: ({
        children,
        render: trigger
    }: PropsWithChildren<{ render?: ReactNode }>) => <>{trigger ?? children}</>
}));

import { LlmEndpointsDialog } from './LlmEndpointsDialog';

describe('LlmEndpointsDialog', () => {
    beforeEach(() => {
        mocks.endpoints = [];
        mocks.load.mockReset();
        mocks.upsert.mockReset();
        mocks.deleteEndpoint.mockReset();
        mocks.detectModels.mockReset();
        mocks.load.mockResolvedValue([]);
        mocks.upsert.mockResolvedValue({});
        mocks.detectModels.mockResolvedValue({
            models: ['openai/o3'],
            modelReasoning: [
                {
                    modelId: 'openai/o3',
                    supportedEfforts: ['low', 'medium', 'high'],
                    mandatory: false
                }
            ]
        });
    });

    afterEach(() => cleanup());

    it('saves automatically detected reasoning metadata for a new endpoint', async () => {
        render(<LlmEndpointsDialog open onOpenChange={vi.fn()} />);

        await waitFor(() => expect(mocks.load).toHaveBeenCalledOnce());
        fireEvent.click(
            screen.getByRole('button', {
                name: 'view.tools.llm_endpoints.add'
            })
        );
        fireEvent.change(
            await screen.findByLabelText('view.tools.llm_endpoints.api_key'),
            { target: { value: 'sk-test' } }
        );
        fireEvent.click(
            screen.getByRole('button', { name: 'common.actions.save' })
        );

        await waitFor(() =>
            expect(mocks.upsert).toHaveBeenCalledWith(
                expect.objectContaining({
                    models: ['openai/o3'],
                    modelReasoning: [
                        {
                            modelId: 'openai/o3',
                            supportedEfforts: ['low', 'medium', 'high'],
                            mandatory: false
                        }
                    ]
                })
            )
        );
    });

    it('preserves existing reasoning metadata when saving without detection', async () => {
        mocks.endpoints = [
            {
                id: 'endpoint-1',
                name: 'OpenAI',
                baseUrl: 'https://api.openai.com/v1',
                hasKey: true,
                models: ['openai/o3'],
                modelReasoning: [
                    {
                        modelId: 'openai/o3',
                        supportedEfforts: ['low', 'medium', 'high'],
                        mandatory: false
                    }
                ],
                lastDetectedAt: '2026-08-07T00:00:00.000Z'
            }
        ];

        render(<LlmEndpointsDialog open onOpenChange={vi.fn()} />);

        await waitFor(() => expect(mocks.load).toHaveBeenCalledOnce());
        fireEvent.click(
            screen.getByRole('button', {
                name: 'view.tools.llm_endpoints.edit'
            })
        );
        fireEvent.click(
            await screen.findByRole('button', {
                name: 'common.actions.save'
            })
        );

        await waitFor(() =>
            expect(mocks.upsert).toHaveBeenCalledWith(
                expect.objectContaining({
                    id: 'endpoint-1',
                    modelReasoning: null
                })
            )
        );
        expect(mocks.detectModels).not.toHaveBeenCalled();
    });
});
