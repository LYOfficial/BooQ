// 文件管理模块

import { invoke } from '@tauri-apps/api/tauri';
import { getPreviewManager, getQuestionManager } from '../main';

export interface FileInfo {
    id: string;
    name: string;
    display_name: string;
    file_type: string;
    path: string;
    size: number;
    created_at: string;
    total_pages: number;
}

export class FileManager {
    private files: FileInfo[] = [];
    private currentFile: FileInfo | null = null;
    private fileToDelete: string | null = null;
    private fileToRename: string | null = null;
    private copiedFile: FileInfo | null = null;

    async init() {
        await this.refreshFileList();
    }

    async uploadFile(filePath: string, fileName: string): Promise<FileInfo | null> {
        try {
            const fileInfo = await invoke<FileInfo>('upload_file', {
                filePath,
                fileName
            });
            return fileInfo;
        } catch (error) {
            console.error('上传文件失败:', error);
            return null;
        }
    }

    async refreshFileList() {
        try {
            this.files = await invoke<FileInfo[]>('get_file_list');
            this.renderFileList();
        } catch (error) {
            console.error('获取文件列表失败:', error);
        }
    }

    private renderFileList() {
        const container = document.getElementById('file-list');
        if (!container) return;

        if (this.files.length === 0) {
            container.innerHTML = `
                <div class="empty-state">
                    <i class="bi bi-folder2-open"></i>
                    <p>暂无文件</p>
                    <p class="text-muted">点击上方按钮上传文件</p>
                </div>
            `;
            return;
        }

        container.innerHTML = this.files.map(file => `
            <div class="file-item ${this.currentFile?.id === file.id ? 'active' : ''}" 
                 data-file-id="${file.id}">
                <i class="bi ${this.getFileIcon(file.file_type)} file-icon"></i>
                <div class="file-info">
                    <div class="file-name" title="${file.display_name}">${file.display_name}</div>
                    <div class="file-size">${this.formatFileSize(file.size)}</div>
                </div>
            </div>
        `).join('');

        // 绑定点击事件
        container.querySelectorAll('.file-item').forEach(item => {
            item.addEventListener('click', (e) => {
                const fileId = (e.currentTarget as HTMLElement).dataset.fileId;
                if (fileId) {
                    this.selectFile(fileId);
                }
            });

            // 绑定右键事件
            item.addEventListener('contextmenu', (e) => {
                e.preventDefault();
                const fileId = (e.currentTarget as HTMLElement).dataset.fileId;
                if (fileId) {
                    this.showContextMenu(fileId, e as MouseEvent);
                }
            });
        });
    }

    private getFileIcon(fileType: string): string {
        const icons: Record<string, string> = {
            'pdf': 'bi-file-earmark-pdf',
            'word': 'bi-file-earmark-word',
            'ppt': 'bi-file-earmark-ppt',
            'txt': 'bi-file-earmark-text',
            'unknown': 'bi-file-earmark'
        };
        return icons[fileType] || icons['unknown'];
    }

    private formatFileSize(bytes: number): string {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    async selectFile(fileId: string) {
        const file = this.files.find(f => f.id === fileId);
        if (!file) return;

        this.currentFile = file;
        this.renderFileList();

        // 通知预览管理器加载文件
        const previewManager = getPreviewManager();
        if (previewManager) {
            await previewManager.loadFile(file);
        }

        // 加载已有的题目
        const questionManager = getQuestionManager();
        if (questionManager) {
            await questionManager.loadQuestions(file.id);
        }
    }

    getCurrentFile(): FileInfo | null {
        return this.currentFile;
    }

    getFiles(): FileInfo[] {
        return this.files;
    }

    getFileById(fileId: string): FileInfo | undefined {
        return this.files.find(f => f.id === fileId);
    }

    private showContextMenu(fileId: string, event: MouseEvent) {
        const file = this.files.find(f => f.id === fileId);
        if (!file) return;

        // 选中该文件
        this.selectFile(fileId);

        // 显示右键菜单
        const menu = document.getElementById('context-menu');
        if (menu) {
            menu.style.display = 'block';
            menu.style.left = `${event.clientX}px`;
            menu.style.top = `${event.clientY}px`;
            menu.dataset.fileId = fileId;
        }
    }

    // 复制文件
    async copyFile(fileId: string) {
        const file = this.files.find(f => f.id === fileId);
        if (file) {
            this.copiedFile = file;
        }
    }

    // 粘贴文件
    async pasteFile() {
        if (!this.copiedFile) return;

        try {
            await invoke('copy_file', { fileId: this.copiedFile.id });
            await this.refreshFileList();
        } catch (error) {
            console.error('粘贴文件失败:', error);
        }
    }

    // 显示删除确认
    showDeleteConfirm(fileId: string) {
        this.fileToDelete = fileId;
        const modal = document.getElementById('delete-confirm-modal');
        if (modal) {
            modal.style.display = 'flex';
        }
    }

    // 隐藏删除确认
    hideDeleteConfirm() {
        this.fileToDelete = null;
        const modal = document.getElementById('delete-confirm-modal');
        if (modal) {
            modal.style.display = 'none';
        }
    }

    // 确认删除
    async confirmDelete() {
        if (!this.fileToDelete) return;

        try {
            await invoke('delete_file', { fileId: this.fileToDelete });
            
            // 如果删除的是当前文件，清除选择
            if (this.currentFile?.id === this.fileToDelete) {
                this.currentFile = null;
            }

            await this.refreshFileList();
        } catch (error) {
            console.error('删除文件失败:', error);
        }

        this.hideDeleteConfirm();
    }

    // 显示重命名对话框
    showRenameDialog(fileId: string) {
        this.fileToRename = fileId;
        const file = this.files.find(f => f.id === fileId);
        if (!file) return;

        const modal = document.getElementById('rename-modal');
        const input = document.getElementById('input-new-name') as HTMLInputElement;
        
        if (modal && input) {
            input.value = file.display_name;
            modal.style.display = 'flex';
            input.focus();
            input.select();
        }
    }

    // 隐藏重命名对话框
    hideRenameDialog() {
        this.fileToRename = null;
        const modal = document.getElementById('rename-modal');
        if (modal) {
            modal.style.display = 'none';
        }
    }

    // 确认重命名
    async confirmRename() {
        if (!this.fileToRename) return;

        const input = document.getElementById('input-new-name') as HTMLInputElement;
        const newName = input?.value?.trim();

        if (!newName) {
            this.hideRenameDialog();
            return;
        }

        try {
            await invoke('rename_file', { 
                fileId: this.fileToRename, 
                newName 
            });
            await this.refreshFileList();
        } catch (error) {
            console.error('重命名失败:', error);
        }

        this.hideRenameDialog();
    }
}
