// 预览管理模块

import { invoke } from '@tauri-apps/api/tauri';
import { FileInfo } from './fileManager';

// 引入 marked 和 katex
declare const marked: any;
declare const katex: any;
declare const renderMathInElement: any;

interface PageContent {
    page_number: number;
    content_type: string;
    content: string;
    width: number;
    height: number;
}

export class PreviewManager {
    private currentFile: FileInfo | null = null;
    private currentPage: number = 1;
    private totalPages: number = 1;
    private mode: 'source' | 'markdown' | 'code' = 'source';
    private markdownCache: Map<number, string> = new Map();

    async init() {
        this.updatePageControls();
    }

    async loadFile(file: FileInfo) {
        this.currentFile = file;
        this.currentPage = 1;
        this.totalPages = file.total_pages;
        this.markdownCache.clear();
        
        this.updatePageControls();
        await this.renderCurrentPage();
    }

    setMode(mode: 'source' | 'markdown' | 'code') {
        this.mode = mode;
        this.renderCurrentPage();
    }

    private updatePageControls() {
        const prevBtn = document.getElementById('btn-prev-page') as HTMLButtonElement;
        const nextBtn = document.getElementById('btn-next-page') as HTMLButtonElement;
        const pageInput = document.getElementById('input-page') as HTMLInputElement;
        const totalPagesSpan = document.getElementById('total-pages');

        if (prevBtn) prevBtn.disabled = this.currentPage <= 1;
        if (nextBtn) nextBtn.disabled = this.currentPage >= this.totalPages;
        if (pageInput) {
            pageInput.value = this.currentPage.toString();
            pageInput.max = this.totalPages.toString();
        }
        if (totalPagesSpan) totalPagesSpan.textContent = this.totalPages.toString();
    }

    async renderCurrentPage() {
        const container = document.getElementById('preview-content');
        if (!container || !this.currentFile) {
            this.showEmptyState();
            return;
        }

        try {
            switch (this.mode) {
                case 'source':
                    await this.renderSourceMode(container);
                    break;
                case 'markdown':
                    await this.renderMarkdownMode(container);
                    break;
                case 'code':
                    await this.renderCodeMode(container);
                    break;
            }
        } catch (error) {
            console.error('渲染页面失败:', error);
            container.innerHTML = `
                <div class="empty-state">
                    <i class="bi bi-exclamation-triangle"></i>
                    <p>加载失败</p>
                    <p class="text-muted">${error}</p>
                </div>
            `;
        }
    }

    private async renderSourceMode(container: HTMLElement) {
        if (!this.currentFile) return;

        const fileType = this.currentFile.file_type;

        if (fileType === 'pdf') {
            await this.renderPDF(container);
        } else if (fileType === 'txt') {
            await this.renderText(container);
        } else {
            container.innerHTML = `
                <div class="empty-state">
                    <i class="bi bi-file-earmark"></i>
                    <p>暂不支持预览此类型文件</p>
                    <p class="text-muted">请使用转换为 Markdown 功能</p>
                </div>
            `;
        }
    }

    private async renderPDF(container: HTMLElement) {
        if (!this.currentFile) return;

        try {
            const pageContent = await invoke<PageContent>('get_file_page', {
                fileId: this.currentFile.id,
                pageNumber: this.currentPage
            });

            // 创建 PDF 预览容器
            container.innerHTML = `
                <div class="pdf-container">
                    <canvas id="pdf-canvas"></canvas>
                </div>
            `;

            // 使用 PDF.js 渲染
            await this.renderPDFWithPDFJS(pageContent.content);
        } catch (error) {
            throw error;
        }
    }

    private async renderPDFWithPDFJS(base64Content: string) {
        // 动态导入 PDF.js
        const pdfjsLib = await import('https://cdn.bootcdn.net/ajax/libs/pdf.js/4.0.379/pdf.min.mjs');
        
        // 设置 worker
        pdfjsLib.GlobalWorkerOptions.workerSrc = 
            'https://cdn.bootcdn.net/ajax/libs/pdf.js/4.0.379/pdf.worker.min.mjs';

        // 解码 base64
        const pdfData = atob(base64Content);
        const pdfArray = new Uint8Array(pdfData.length);
        for (let i = 0; i < pdfData.length; i++) {
            pdfArray[i] = pdfData.charCodeAt(i);
        }

        // 加载 PDF
        const loadingTask = pdfjsLib.getDocument({ data: pdfArray });
        const pdf = await loadingTask.promise;

        // 渲染页面
        const page = await pdf.getPage(this.currentPage);
        const scale = 1.5;
        const viewport = page.getViewport({ scale });

        const canvas = document.getElementById('pdf-canvas') as HTMLCanvasElement;
        if (!canvas) return;

        const context = canvas.getContext('2d');
        if (!context) return;

        canvas.height = viewport.height;
        canvas.width = viewport.width;

        const renderContext = {
            canvasContext: context,
            viewport: viewport
        };

        await page.render(renderContext).promise;
    }

    private async renderText(container: HTMLElement) {
        if (!this.currentFile) return;

        const pageContent = await invoke<PageContent>('get_file_page', {
            fileId: this.currentFile.id,
            pageNumber: this.currentPage
        });

        container.innerHTML = `
            <div class="text-preview">${this.escapeHtml(pageContent.content)}</div>
        `;
        container.classList.add('text-mode');
    }

    private async renderMarkdownMode(container: HTMLElement) {
        if (!this.currentFile) return;

        // 检查缓存
        let markdown = this.markdownCache.get(this.currentPage);
        
        if (!markdown) {
            try {
                markdown = await invoke<string>('get_markdown_content', {
                    fileId: this.currentFile.id,
                    pageNumber: this.currentPage
                });
                this.markdownCache.set(this.currentPage, markdown);
            } catch (error) {
                // 如果没有缓存，提示用户转换
                container.innerHTML = `
                    <div class="empty-state">
                        <i class="bi bi-file-earmark-text"></i>
                        <p>尚未转换为 Markdown</p>
                        <p class="text-muted">请点击"转换成Markdown格式"按钮</p>
                    </div>
                `;
                return;
            }
        }

        // 渲染 Markdown
        const html = marked.parse(markdown);
        container.innerHTML = `<div class="markdown-body">${html}</div>`;
        container.classList.add('text-mode');

        // 渲染数学公式
        if (typeof renderMathInElement !== 'undefined') {
            renderMathInElement(container, {
                delimiters: [
                    { left: '$$', right: '$$', display: true },
                    { left: '$', right: '$', display: false },
                    { left: '\\[', right: '\\]', display: true },
                    { left: '\\(', right: '\\)', display: false }
                ],
                throwOnError: false
            });
        }
    }

    private async renderCodeMode(container: HTMLElement) {
        if (!this.currentFile) return;

        // 检查缓存
        let markdown = this.markdownCache.get(this.currentPage);
        
        if (!markdown) {
            try {
                markdown = await invoke<string>('get_markdown_source', {
                    fileId: this.currentFile.id,
                    pageNumber: this.currentPage
                });
                this.markdownCache.set(this.currentPage, markdown);
            } catch (error) {
                container.innerHTML = `
                    <div class="empty-state">
                        <i class="bi bi-code-slash"></i>
                        <p>尚未转换为 Markdown</p>
                        <p class="text-muted">请先点击"转换成Markdown格式"按钮</p>
                    </div>
                `;
                return;
            }
        }

        container.innerHTML = `<pre class="code-view">${this.escapeHtml(markdown)}</pre>`;
        container.classList.add('text-mode');
    }

    private showEmptyState() {
        const container = document.getElementById('preview-content');
        if (container) {
            container.innerHTML = `
                <div class="empty-state">
                    <i class="bi bi-file-earmark-richtext"></i>
                    <p>选择左侧文件进行预览</p>
                </div>
            `;
        }
    }

    async convertToMarkdown() {
        if (!this.currentFile) return;

        try {
            const markdown = await invoke<string>('convert_page_to_markdown', {
                fileId: this.currentFile.id,
                pageNumber: this.currentPage
            });
            
            this.markdownCache.set(this.currentPage, markdown);
            
            // 切换到 Markdown 模式
            this.mode = 'markdown';
            document.querySelectorAll('.tab-btn').forEach(btn => {
                btn.classList.remove('active');
                if ((btn as HTMLElement).dataset.mode === 'markdown') {
                    btn.classList.add('active');
                }
            });
            
            await this.renderCurrentPage();
        } catch (error) {
            console.error('转换 Markdown 失败:', error);
        }
    }

    async prevPage() {
        if (this.currentPage > 1) {
            this.currentPage--;
            this.updatePageControls();
            await this.renderCurrentPage();
        }
    }

    async nextPage() {
        if (this.currentPage < this.totalPages) {
            this.currentPage++;
            this.updatePageControls();
            await this.renderCurrentPage();
        }
    }

    async goToPage(page: number) {
        if (page >= 1 && page <= this.totalPages) {
            this.currentPage = page;
            this.updatePageControls();
            await this.renderCurrentPage();
        }
    }

    getCurrentPage(): number {
        return this.currentPage;
    }

    getTotalPages(): number {
        return this.totalPages;
    }

    private escapeHtml(text: string): string {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}
