export const TILE_SHELL =
    'group/tile pointer-fine:hover:border-foreground/25 bg-muted/40 relative h-auto w-full min-w-0 overflow-hidden rounded-lg border p-0 aria-disabled:cursor-not-allowed';

export const TILE_MOTION =
    'duration-150 ease-[cubic-bezier(0.23,1,0.32,1)] active:scale-[0.97] aria-disabled:active:translate-y-0 aria-disabled:active:scale-100';

export const TILE_MOTION_STANDALONE = `transition-[color,background-color,border-color,box-shadow,opacity,transform] motion-reduce:transition-colors ${TILE_MOTION}`;

export const TILE_SURFACE =
    'bg-muted/40 text-muted-foreground pointer-fine:group-hover/tile:bg-muted/55 flex size-full items-center justify-center overflow-hidden transition-colors duration-150';

export const TILE_SELECTED = 'ring-primary ring-2';

export const TILE_LOCKED = 'opacity-70 grayscale-[0.6]';

export const TILE_LABEL =
    'pointer-events-none absolute inset-x-0 bottom-0 truncate bg-gradient-to-t from-black/85 via-black/45 to-transparent px-2 pt-6 pb-1.5 text-left text-xs font-semibold text-white';

export const TILE_CHECK =
    'bg-primary text-primary-foreground ring-background absolute end-1.5 top-1.5 flex size-5 items-center justify-center rounded-full shadow-sm ring-2';

export const TILE_BADGE =
    'bg-black/70 absolute start-1.5 top-1.5 flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[10px] font-semibold text-amber-300 backdrop-blur-sm';

export const TILE_BUSY_OVERLAY =
    'bg-background/55 text-foreground absolute inset-0 flex items-center justify-center backdrop-blur-[1px]';
