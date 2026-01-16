// 全局类型声明

// Marked.js
declare const marked: {
    parse(markdown: string): string;
};

// KaTeX
declare const katex: {
    render(tex: string, element: HTMLElement, options?: object): void;
    renderToString(tex: string, options?: object): string;
};

declare function renderMathInElement(element: HTMLElement, options?: {
    delimiters?: Array<{
        left: string;
        right: string;
        display: boolean;
    }>;
    throwOnError?: boolean;
}): void;

// PDF.js
declare module 'https://cdn.bootcdn.net/ajax/libs/pdf.js/4.0.379/pdf.min.mjs' {
    export const GlobalWorkerOptions: {
        workerSrc: string;
    };
    
    export function getDocument(options: { data: Uint8Array }): {
        promise: Promise<PDFDocument>;
    };
    
    interface PDFDocument {
        getPage(pageNumber: number): Promise<PDFPage>;
        numPages: number;
    }
    
    interface PDFPage {
        getViewport(options: { scale: number }): PDFViewport;
        render(options: { canvasContext: CanvasRenderingContext2D; viewport: PDFViewport }): { promise: Promise<void> };
    }
    
    interface PDFViewport {
        width: number;
        height: number;
    }
}
