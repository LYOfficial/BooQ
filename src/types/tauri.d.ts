// Tauri API 类型声明

declare module '@tauri-apps/api/tauri' {
    export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
}

declare module '@tauri-apps/api/window' {
    export const appWindow: {
        minimize(): Promise<void>;
        maximize(): Promise<void>;
        unmaximize(): Promise<void>;
        isMaximized(): Promise<boolean>;
        close(): Promise<void>;
    };
}

declare module '@tauri-apps/api/dialog' {
    export interface OpenDialogOptions {
        multiple?: boolean;
        directory?: boolean;
        filters?: { name: string; extensions: string[] }[];
        title?: string;
    }
    
    export function open(options?: OpenDialogOptions): Promise<string | string[] | null>;
}

declare module '@tauri-apps/api/event' {
    export function listen<T>(event: string, handler: (event: { payload: T }) => void): Promise<() => void>;
}
