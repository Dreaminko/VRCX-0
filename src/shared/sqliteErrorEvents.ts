type SQLiteErrorListener = (error: Error) => void;

const sqliteErrorListeners = new Set<SQLiteErrorListener>();

export function subscribeSQLiteError(listener: SQLiteErrorListener): () => void {
    sqliteErrorListeners.add(listener);
    return () => {
        sqliteErrorListeners.delete(listener);
    };
}

export function notifySQLiteError(error: Error): void {
    for (const listener of sqliteErrorListeners) {
        try {
            listener(error);
        } catch (listenerError) {
            console.warn('SQLite error listener failed:', listenerError);
        }
    }
}
