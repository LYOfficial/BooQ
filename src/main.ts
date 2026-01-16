// BooQ 前端主入口
// TypeScript + Tauri API

import { invoke } from '@tauri-apps/api/tauri';
import { appWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/api/dialog';
import { listen } from '@tauri-apps/api/event';

import { FileManager } from './modules/fileManager';
import { PreviewManager } from './modules/previewManager';
import { QuestionManager } from './modules/questionManager';
import { SettingsManager } from './modules/settingsManager';
import { ThemeManager } from './modules/themeManager';
import { ContextMenu } from './modules/contextMenu';

// 全局实例
let fileManager: FileManager;
let previewManager: PreviewManager;
let questionManager: QuestionManager;
let settingsManager: SettingsManager;
let themeManager: ThemeManager;
let contextMenu: ContextMenu;

// 初始化应用
async function initApp() {
    console.log('BooQ 正在初始化...');

    // 初始化主题管理器
    themeManager = new ThemeManager();
    await themeManager.init();

    // 初始化设置管理器
    settingsManager = new SettingsManager();
    await settingsManager.init();

    // 初始化文件管理器
    fileManager = new FileManager();
    await fileManager.init();

    // 初始化预览管理器
    previewManager = new PreviewManager();
    await previewManager.init();

    // 初始化题目管理器
    questionManager = new QuestionManager();
    await questionManager.init();

    // 初始化右键菜单
    contextMenu = new ContextMenu(fileManager);
    contextMenu.init();

    // 绑定标题栏按钮事件
    bindTitlebarEvents();

    // 绑定文件操作事件
    bindFileEvents();

    // 绑定预览操作事件
    bindPreviewEvents();

    // 绑定设置事件
    bindSettingsEvents();

    console.log('BooQ 初始化完成');
}

// 绑定标题栏按钮事件
function bindTitlebarEvents() {
    // 最小化按钮
    document.getElementById('btn-minimize')?.addEventListener('click', () => {
        appWindow.minimize();
    });

    // 最大化/还原按钮
    document.getElementById('btn-maximize')?.addEventListener('click', async () => {
        const isMaximized = await appWindow.isMaximized();
        if (isMaximized) {
            appWindow.unmaximize();
        } else {
            appWindow.maximize();
        }
    });

    // 关闭按钮
    document.getElementById('btn-close')?.addEventListener('click', () => {
        appWindow.close();
    });

    // 主题切换按钮
    document.getElementById('btn-theme')?.addEventListener('click', () => {
        themeManager.cycleTheme();
    });

    // 设置按钮
    document.getElementById('btn-settings')?.addEventListener('click', () => {
        settingsManager.showSettings();
    });
}

// 绑定文件操作事件
function bindFileEvents() {
    // 上传文件按钮
    document.getElementById('btn-upload')?.addEventListener('click', async () => {
        const selected = await open({
            multiple: true,
            filters: [{
                name: '文档文件',
                extensions: ['pdf', 'doc', 'docx', 'ppt', 'pptx', 'txt']
            }]
        });

        if (selected) {
            const files = Array.isArray(selected) ? selected : [selected];
            for (const filePath of files) {
                const fileName = filePath.split(/[/\\]/).pop() || 'unknown';
                await fileManager.uploadFile(filePath, fileName);
            }
            await fileManager.refreshFileList();
        }
    });
}

// 绑定预览操作事件
function bindPreviewEvents() {
    // 预览模式切换
    document.querySelectorAll('.tab-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const target = e.currentTarget as HTMLElement;
            const mode = target.dataset.mode;
            if (mode) {
                document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
                target.classList.add('active');
                previewManager.setMode(mode as 'source' | 'markdown' | 'code');
            }
        });
    });

    // 转换为 Markdown 按钮
    document.getElementById('btn-convert-md')?.addEventListener('click', async () => {
        await previewManager.convertToMarkdown();
    });

    // 重新识别按钮
    document.getElementById('btn-refresh-ocr')?.addEventListener('click', async () => {
        await previewManager.refreshCurrentPage();
    });

    // 开始分析按钮
    document.getElementById('btn-start-analysis')?.addEventListener('click', async () => {
        const currentFile = fileManager.getCurrentFile();
        if (currentFile) {
            await questionManager.startAnalysis(currentFile.id);
        }
    });

    // 翻页按钮
    document.getElementById('btn-prev-page')?.addEventListener('click', () => {
        previewManager.prevPage();
    });

    document.getElementById('btn-next-page')?.addEventListener('click', () => {
        previewManager.nextPage();
    });

    // 页码输入
    document.getElementById('input-page')?.addEventListener('change', (e) => {
        const target = e.target as HTMLInputElement;
        const page = parseInt(target.value);
        if (!isNaN(page)) {
            previewManager.goToPage(page);
        }
    });

    // 题目翻页
    document.getElementById('btn-prev-question')?.addEventListener('click', () => {
        questionManager.prevQuestion();
    });

    document.getElementById('btn-next-question')?.addEventListener('click', () => {
        questionManager.nextQuestion();
    });
}

// 绑定设置事件
function bindSettingsEvents() {
    // 关闭设置弹窗
    document.getElementById('btn-close-settings')?.addEventListener('click', () => {
        settingsManager.hideSettings();
    });

    document.getElementById('btn-cancel-settings')?.addEventListener('click', () => {
        settingsManager.hideSettings();
    });

    // 保存设置
    document.getElementById('btn-save-settings')?.addEventListener('click', async () => {
        await settingsManager.saveSettings();
    });

    // 浏览存储路径
    document.getElementById('btn-browse-path')?.addEventListener('click', async () => {
        await settingsManager.browseStoragePath();
    });

    // 主题选择
    document.querySelectorAll('input[name="theme"]').forEach(input => {
        input.addEventListener('change', (e) => {
            const target = e.target as HTMLInputElement;
            themeManager.setTheme(target.value as 'light' | 'dark' | 'system');
        });
    });

    // 添加模型
    document.getElementById('btn-add-model')?.addEventListener('click', () => {
        settingsManager.showAddModelDialog();
    });

    document.getElementById('btn-close-add-model')?.addEventListener('click', () => {
        settingsManager.hideAddModelDialog();
    });

    document.getElementById('btn-cancel-add-model')?.addEventListener('click', () => {
        settingsManager.hideAddModelDialog();
    });

    document.getElementById('btn-confirm-add-model')?.addEventListener('click', async () => {
        await settingsManager.addModel();
    });

    // 删除确认
    document.getElementById('btn-cancel-delete')?.addEventListener('click', () => {
        fileManager.hideDeleteConfirm();
    });

    document.getElementById('btn-confirm-delete')?.addEventListener('click', async () => {
        await fileManager.confirmDelete();
    });

    // 重命名
    document.getElementById('btn-cancel-rename')?.addEventListener('click', () => {
        fileManager.hideRenameDialog();
    });

    document.getElementById('btn-confirm-rename')?.addEventListener('click', async () => {
        await fileManager.confirmRename();
    });

    // 点击模态框遮罩层关闭模态框
    document.querySelectorAll('.modal-overlay').forEach(overlay => {
        overlay.addEventListener('click', (e) => {
            // 只有点击遮罩层本身时才关闭，点击对话框内部不关闭
            if (e.target === overlay) {
                (overlay as HTMLElement).style.display = 'none';
            }
        });
    });
}

// 导出全局访问接口
export function getFileManager() { return fileManager; }
export function getPreviewManager() { return previewManager; }
export function getQuestionManager() { return questionManager; }
export function getSettingsManager() { return settingsManager; }
export function getThemeManager() { return themeManager; }

// 启动应用
document.addEventListener('DOMContentLoaded', initApp);
