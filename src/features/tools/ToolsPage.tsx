import { ToolsPageContent } from './components/ToolsPageContent';

export function ToolsPage({ embedded = false }: { embedded?: boolean } = {}) {
    return <ToolsPageContent embedded={embedded} />;
}
