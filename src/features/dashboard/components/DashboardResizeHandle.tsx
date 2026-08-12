import { ResizableHandle } from '@/ui/shadcn/resizable';

export function DashboardResizeHandle() {
    return (
        <ResizableHandle className="bg-border hover:bg-primary/70 focus-visible:bg-primary/70 z-10 w-0.5 shrink-0 cursor-col-resize transition-colors duration-150 after:w-2 aria-[orientation=horizontal]:h-0.5 aria-[orientation=horizontal]:w-full aria-[orientation=horizontal]:cursor-row-resize aria-[orientation=horizontal]:after:h-2 aria-[orientation=horizontal]:after:w-full" />
    );
}
