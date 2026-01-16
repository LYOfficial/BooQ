// 右键菜单模块

import { FileManager } from './fileManager';

export class ContextMenu {
    private fileManager: FileManager;
    private menu: HTMLElement | null = null;

    constructor(fileManager: FileManager) {
        this.fileManager = fileManager;
    }

    init() {
        this.menu = document.getElementById('context-menu');
        
        // 点击其他地方关闭菜单
        document.addEventListener('click', (e) => {
            if (this.menu && !this.menu.contains(e.target as Node)) {
                this.hide();
            }
        });

        // 绑定菜单项事件
        this.bindMenuItems();
    }

    private bindMenuItems() {
        if (!this.menu) return;

        this.menu.querySelectorAll('.context-menu-item').forEach(item => {
            item.addEventListener('click', (e) => {
                const action = (e.currentTarget as HTMLElement).dataset.action;
                const fileId = this.menu?.dataset.fileId;
                
                if (action && fileId) {
                    this.handleAction(action, fileId);
                }
                
                this.hide();
            });
        });
    }

    private handleAction(action: string, fileId: string) {
        switch (action) {
            case 'copy':
                this.fileManager.copyFile(fileId);
                break;
            case 'paste':
                this.fileManager.pasteFile();
                break;
            case 'rename':
                this.fileManager.showRenameDialog(fileId);
                break;
            case 'delete':
                this.fileManager.showDeleteConfirm(fileId);
                break;
        }
    }

    show(x: number, y: number, fileId: string) {
        if (!this.menu) return;

        this.menu.style.display = 'block';
        this.menu.style.left = `${x}px`;
        this.menu.style.top = `${y}px`;
        this.menu.dataset.fileId = fileId;

        // 确保菜单不超出窗口边界
        const rect = this.menu.getBoundingClientRect();
        if (rect.right > window.innerWidth) {
            this.menu.style.left = `${x - rect.width}px`;
        }
        if (rect.bottom > window.innerHeight) {
            this.menu.style.top = `${y - rect.height}px`;
        }
    }

    hide() {
        if (this.menu) {
            this.menu.style.display = 'none';
        }
    }
}
