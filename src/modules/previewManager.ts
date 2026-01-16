// 预览管理模块

import { invoke } from '@tauri-apps/api/tauri';
import { FileInfo } from './fileManager';

// 引入 marked 和 katex
declare const marked: any;
declare const katex: any;
declare const renderMathInElement: any;
declare const pdfjsLib: any;

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
    private isLoading: boolean = false;
    private pendingOCRPages: Set<number> = new Set();

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
        const previousMode = this.mode;
        this.mode = mode;
        
        // 如果切换到 markdown 或 code 模式，自动触发 OCR
        if ((mode === 'markdown' || mode === 'code') && previousMode === 'source') {
            this.renderCurrentPageWithOCR();
        } else {
            this.renderCurrentPage();
        }
    }

    // 显示加载状态
    private showLoading(container: HTMLElement, message: string = '正在识别中，请稍候...') {
        this.isLoading = true;
        container.innerHTML = `
            <div class="loading-state">
                <div class="loading-spinner"></div>
                <p class="loading-text">${message}</p>
                <p class="loading-hint">正在使用 AI 进行文档解析</p>
            </div>
        `;
    }

    // 隐藏加载状态
    private hideLoading() {
        this.isLoading = false;
    }

    // 带 OCR 的页面渲染
    private async renderCurrentPageWithOCR() {
        const container = document.getElementById('preview-content');
        if (!container || !this.currentFile) {
            this.showEmptyState();
            return;
        }

        // 检查缓存
        if (this.markdownCache.has(this.currentPage)) {
            await this.renderCurrentPage();
            return;
        }

        // 显示加载动画
        this.showLoading(container, `正在识别第 ${this.currentPage} 页...`);

        try {
            // 调用 OCR 转换
            const markdown = await invoke<string>('convert_page_to_markdown', {
                fileId: this.currentFile.id,
                pageNumber: this.currentPage
            });
            
            // 保存到缓存
            this.markdownCache.set(this.currentPage, markdown);
            
            // 渲染页面
            this.hideLoading();
            await this.renderCurrentPage();
        } catch (error) {
            console.error('OCR 识别失败:', error);
            this.hideLoading();
            container.innerHTML = `
                <div class="empty-state error-state">
                    <i class="bi bi-exclamation-triangle"></i>
                    <p>识别失败</p>
                    <p class="text-muted">${error}</p>
                    <button class="btn btn-primary btn-sm mt-3" onclick="window.retryOCR && window.retryOCR()">
                        <i class="bi bi-arrow-clockwise"></i> 重试
                    </button>
                </div>
            `;
            // 暴露重试方法
            (window as any).retryOCR = () => this.renderCurrentPageWithOCR();
        }
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
        // 使用全局 pdfjsLib
        if (typeof pdfjsLib === 'undefined') {
            throw new Error('PDF.js 库未加载');
        }

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
            // 没有缓存，显示加载动画并触发 OCR
            this.showLoading(container, `正在识别第 ${this.currentPage} 页...`);
            
            try {
                markdown = await invoke<string>('convert_page_to_markdown', {
                    fileId: this.currentFile.id,
                    pageNumber: this.currentPage
                });
                // 规范化 LaTeX 代码
                markdown = this.normalizeLatex(markdown);
                this.markdownCache.set(this.currentPage, markdown);
                this.hideLoading();
            } catch (error) {
                console.error('OCR 识别失败:', error);
                this.hideLoading();
                container.innerHTML = `
                    <div class="empty-state error-state">
                        <i class="bi bi-exclamation-triangle"></i>
                        <p>识别失败</p>
                        <p class="text-muted">${error}</p>
                        <button class="btn btn-primary btn-sm mt-3" onclick="window.retryOCR && window.retryOCR()">
                            <i class="bi bi-arrow-clockwise"></i> 重试
                        </button>
                    </div>
                `;
                (window as any).retryOCR = () => this.renderMarkdownMode(container);
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

    // LaTeX 规范化处理
    private normalizeLatex(markdown: string): string {
        let result = markdown;
        
        // 1. 修复独立的 \begin{...} 块，确保被 $$ 包裹
        const blockEnvs = ['aligned', 'equation', 'gather', 'align', 'split', 'cases', 'matrix', 'pmatrix', 'bmatrix', 'vmatrix', 'array'];
        
        for (const env of blockEnvs) {
            // 匹配 \begin{env}...\end{env} 块
            const blockRegex = new RegExp(
                `\\\\begin\\{${env}\\}([\\s\\S]*?)\\\\end\\{${env}\\}`,
                'g'
            );
            
            result = result.replace(blockRegex, (match) => {
                // 检查这个块是否已经被 $$ 包裹
                const beforeMatch = result.substring(0, result.indexOf(match));
                const lastDollarIndex = beforeMatch.lastIndexOf('$$');
                const isWrapped = lastDollarIndex !== -1 && 
                    beforeMatch.substring(lastDollarIndex).match(/\$\$\s*$/);
                
                if (!isWrapped) {
                    // 修复内部换行：将 \\ 后的换行确保正确
                    let fixed = match.replace(/\\\\\s*\n?\s*/g, '\\\\\n');
                    return `\n$$\n${fixed}\n$$\n`;
                }
                return match;
            });
        }
        
        // 2. 清理多余的 $$ 符号（可能因上面的替换产生重复）
        result = result.replace(/\$\$\s*\$\$/g, '$$');
        result = result.replace(/\$\$\n\$\$/g, '$$');
        
        // 3. 修复公式块内的反斜杠问题
        // 确保 aligned 等环境内的 \\ 是正确的
        result = result.replace(/\\\\\\\\/g, '\\\\');
        
        // 4. 修复公式中的中文标点
        result = result.replace(/(\$[^$]+)。([^$]*\$)/g, '$1.$2');
        result = result.replace(/(\$[^$]+)，([^$]*\$)/g, '$1,$2');
        result = result.replace(/(\$[^$]+)％([^$]*\$)/g, '$1\\%$2');
        
        // 5. 确保 $$ 块前后有换行，但避免多余空行
        result = result.replace(/([^\n\s])\$\$/g, '$1\n$$');
        result = result.replace(/\$\$([^\n\s$])/g, '$$\n$1');
        
        // 6. 修复百分号 % 在 LaTeX 中是注释符
        result = result.replace(/(\d)%(?!\s*\$)/g, '$1\\%');
        result = result.replace(/(\d)％/g, '$1\\%');
        
        // 7. 清理多余的空行
        result = result.replace(/\n{3,}/g, '\n\n');
        
        // 8. 修复 \times 格式
        result = result.replace(/\\times(\d)/g, '\\times $1');
        result = result.replace(/(\d)\\times/g, '$1 \\times');
        
        // 9. 修复 &= 等对齐符号前后的空格
        result = result.replace(/\s*&\s*=/g, ' &= ');
        result = result.replace(/\s*&\s*</g, ' &< ');
        result = result.replace(/\s*&\s*>/g, ' &> ');
        
        return result;
    }

    private async renderCodeMode(container: HTMLElement) {
        if (!this.currentFile) return;

        // 检查缓存
        let markdown = this.markdownCache.get(this.currentPage);
        
        if (!markdown) {
            // 没有缓存，显示加载动画并触发 OCR
            this.showLoading(container, `正在识别第 ${this.currentPage} 页...`);
            
            try {
                markdown = await invoke<string>('convert_page_to_markdown', {
                    fileId: this.currentFile.id,
                    pageNumber: this.currentPage
                });
                // 规范化 LaTeX 代码
                markdown = this.normalizeLatex(markdown);
                this.markdownCache.set(this.currentPage, markdown);
                this.hideLoading();
            } catch (error) {
                console.error('OCR 识别失败:', error);
                this.hideLoading();
                container.innerHTML = `
                    <div class="empty-state error-state">
                        <i class="bi bi-exclamation-triangle"></i>
                        <p>识别失败</p>
                        <p class="text-muted">${error}</p>
                        <button class="btn btn-primary btn-sm mt-3" onclick="window.retryOCR && window.retryOCR()">
                            <i class="bi bi-arrow-clockwise"></i> 重试
                        </button>
                    </div>
                `;
                (window as any).retryOCR = () => this.renderCodeMode(container);
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
            // 在 markdown/code 模式下，翻页时自动触发 OCR
            if ((this.mode === 'markdown' || this.mode === 'code') && !this.markdownCache.has(this.currentPage)) {
                await this.renderCurrentPageWithOCR();
            } else {
                await this.renderCurrentPage();
            }
        }
    }

    async nextPage() {
        if (this.currentPage < this.totalPages) {
            this.currentPage++;
            this.updatePageControls();
            // 在 markdown/code 模式下，翻页时自动触发 OCR
            if ((this.mode === 'markdown' || this.mode === 'code') && !this.markdownCache.has(this.currentPage)) {
                await this.renderCurrentPageWithOCR();
            } else {
                await this.renderCurrentPage();
            }
        }
    }

    async goToPage(page: number) {
        if (page >= 1 && page <= this.totalPages) {
            this.currentPage = page;
            this.updatePageControls();
            // 在 markdown/code 模式下，跳转页面时自动触发 OCR
            if ((this.mode === 'markdown' || this.mode === 'code') && !this.markdownCache.has(this.currentPage)) {
                await this.renderCurrentPageWithOCR();
            } else {
                await this.renderCurrentPage();
            }
        }
    }

    getCurrentPage(): number {
        return this.currentPage;
    }

    getTotalPages(): number {
        return this.totalPages;
    }

    // 清除当前页面缓存并重新识别
    async refreshCurrentPage() {
        if (!this.currentFile) return;
        
        const container = document.getElementById('preview-content');
        if (!container) return;
        
        // 清除前端缓存
        this.markdownCache.delete(this.currentPage);
        
        // 清除后端缓存
        try {
            await invoke('clear_markdown_cache', {
                fileId: this.currentFile.id,
                pageNumber: this.currentPage
            });
        } catch (error) {
            console.error('清除缓存失败:', error);
        }
        
        // 重新识别
        if (this.mode === 'markdown' || this.mode === 'code') {
            await this.renderCurrentPageWithOCR();
        } else {
            await this.renderCurrentPage();
        }
    }

    // 清除所有页面缓存
    async clearAllCache() {
        if (!this.currentFile) return;
        
        // 清除前端缓存
        this.markdownCache.clear();
        
        // 清除后端缓存
        try {
            await invoke('clear_markdown_cache', {
                fileId: this.currentFile.id,
                pageNumber: null
            });
        } catch (error) {
            console.error('清除缓存失败:', error);
        }
    }

    private escapeHtml(text: string): string {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
}
