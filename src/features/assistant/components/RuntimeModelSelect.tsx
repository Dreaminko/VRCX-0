import { useLlmEndpointsStore } from '@/state/llmEndpointsStore';
import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectLabel,
    SelectTrigger,
    SelectValue
} from '@/ui/shadcn/select';

const RUNTIME_SEPARATOR = '::';

export type RuntimeModelRef = {
    endpointId: string;
    model: string;
};

function runtimeModelValue(endpointId: string, model: string): string {
    return `${endpointId}${RUNTIME_SEPARATOR}${model}`;
}

function parseRuntimeModelValue(value: string): RuntimeModelRef | null {
    const separatorIndex = value.indexOf(RUNTIME_SEPARATOR);
    if (separatorIndex < 0) {
        return null;
    }
    return {
        endpointId: value.slice(0, separatorIndex),
        model: value.slice(separatorIndex + RUNTIME_SEPARATOR.length)
    };
}

type RuntimeModelSelectProps = {
    endpointId: string | null;
    model: string | null;
    placeholder: string;
    triggerId?: string;
    onSelect: (ref: RuntimeModelRef) => void;
};

export function RuntimeModelSelect({
    endpointId,
    model,
    placeholder,
    triggerId,
    onSelect
}: RuntimeModelSelectProps) {
    const endpoints = useLlmEndpointsStore((state) => state.endpoints);
    const items = endpoints.flatMap((endpoint) =>
        endpoint.models.map((endpointModel) => ({
            value: runtimeModelValue(endpoint.id, endpointModel),
            label: endpointModel
        }))
    );
    const value =
        endpointId && model ? runtimeModelValue(endpointId, model) : undefined;

    function handleValueChange(next: string | null) {
        const parsed = next ? parseRuntimeModelValue(next) : null;
        if (parsed) {
            onSelect(parsed);
        }
    }

    return (
        <Select
            value={value}
            items={items}
            disabled={!items.length}
            onValueChange={handleValueChange}
        >
            <SelectTrigger id={triggerId} className="w-full">
                <SelectValue placeholder={placeholder} />
            </SelectTrigger>
            <SelectContent>
                {endpoints.map((endpoint) => (
                    <SelectGroup key={endpoint.id}>
                        <SelectLabel>{endpoint.name}</SelectLabel>
                        {endpoint.models.map((endpointModel) => (
                            <SelectItem
                                key={endpointModel}
                                value={runtimeModelValue(
                                    endpoint.id,
                                    endpointModel
                                )}
                            >
                                {endpointModel}
                            </SelectItem>
                        ))}
                    </SelectGroup>
                ))}
            </SelectContent>
        </Select>
    );
}
