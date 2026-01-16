// 设置管理模块

import { invoke } from '@tauri-apps/api/tauri';
import { open } from '@tauri-apps/api/dialog';
import { listen } from '@tauri-apps/api/event';
import { getThemeManager } from '../main';

// 定义解除监听函数类型
type UnlistenFn = () => void;

// 日志条目接口
export interface LogEntry {
    timestamp: string;
    level: string;
    source: string;
    message: string;
}

// MinerU 安装信息接口
export interface MineruInstallInfo {
    is_installed: boolean;
    command_available: boolean;
    executable_path: string | null;
    models_downloaded: boolean;
    ocr_models_downloaded: boolean;
    models_dir: string | null;
    modelscope_installed: boolean;
}

export interface ModelConfig {
    id: string;
    name: string;
    provider: string;
    api_url: string;
    api_key: string;
    model_name: string;
}

export interface AppConfig {
    storage_path: string;
    theme: string;
    models: ModelConfig[];
    reading_model: string;
    analysis_model: string;
    solving_model: string;
    // OCR 相关配置
    use_paddle_ocr: boolean;
    mineru_installed: boolean;
    paddle_ocr_url: string;
    paddle_ocr_token: string;
}

// 供应商配置信息
const PROVIDER_CONFIG: Record<string, { name: string; defaultUrl: string; icon: string }> = {
    openai: {
        name: 'OpenAI',
        defaultUrl: 'https://api.openai.com/v1/chat/completions',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/openai.png'
    },
    anthropic: {
        name: 'Anthropic',
        defaultUrl: 'https://api.anthropic.com/v1/messages',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/anthropic.png'
    },
    gemini: {
        name: 'Google Gemini',
        defaultUrl: 'https://generativelanguage.googleapis.com/v1beta/models',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/gemini.png'
    },
    deepseek: {
        name: 'DeepSeek',
        defaultUrl: 'https://api.deepseek.com/v1/chat/completions',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/deepseek.png'
    },
    zhipu: {
        name: '智谱AI',
        defaultUrl: 'https://open.bigmodel.cn/api/paas/v4/chat/completions',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/zhipu.png'
    },
    qwen: {
        name: '通义千问',
        defaultUrl: 'https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/qwen.png'
    },
    moonshot: {
        name: 'Moonshot',
        defaultUrl: 'https://api.moonshot.cn/v1/chat/completions',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/moonshot.png'
    },
    ollama: {
        name: 'Ollama',
        defaultUrl: 'http://localhost:11434/api/chat',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/ollama.png'
    },
    siliconcloud: {
        name: '硅基流动',
        defaultUrl: 'https://api.siliconflow.cn/v1/chat/completions',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/siliconcloud.png'
    },
    doubao: {
        name: '豆包',
        defaultUrl: 'https://ark.cn-beijing.volces.com/api/v3/chat/completions',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/doubao.png'
    },
    groq: {
        name: 'Groq',
        defaultUrl: 'https://api.groq.com/openai/v1/chat/completions',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/groq.png'
    },
    azure: {
        name: 'Azure OpenAI',
        defaultUrl: 'https://YOUR_RESOURCE.openai.azure.com/openai/deployments/YOUR_DEPLOYMENT/chat/completions?api-version=2024-02-01',
        icon: 'https://registry.npmmirror.com/@lobehub/icons-static-png/latest/files/light/azure.png'
    },
    custom: {
        name: '自定义',
        defaultUrl: '',
        icon: ''
    }
};

export class SettingsManager {
    private config: AppConfig | null = null;
    private selectedProvider: string = '';

    async init() {
        await this.loadConfig();
        this.bindNavigationEvents();
        this.bindProviderCardEvents();
        this.bindLogEvents();
    }

    private bindNavigationEvents() {
        const navItems = document.querySelectorAll('.settings-nav-item');
        navItems.forEach(item => {
            item.addEventListener('click', () => {
                const section = item.getAttribute('data-section');
                if (section) {
                    this.switchSection(section);
                }
            });
        });
    }

    private switchSection(section: string) {
        // 更新导航项
        document.querySelectorAll('.settings-nav-item').forEach(item => {
            item.classList.toggle('active', item.getAttribute('data-section') === section);
        });

        // 更新内容面板
        document.querySelectorAll('.settings-section-panel').forEach(panel => {
            panel.classList.toggle('active', panel.id === `section-${section}`);
        });

        // 切换到日志面板时自动加载日志
        if (section === 'logs') {
            this.loadLogs();
        }

        // 切换到工具面板时自动检测 MinerU 状态
        if (section === 'tools') {
            this.checkMinerU();
        }
    }

    private bindProviderCardEvents() {
        const providerGrid = document.getElementById('provider-grid');
        if (providerGrid) {
            providerGrid.addEventListener('click', (e) => {
                const card = (e.target as HTMLElement).closest('.provider-card');
                if (card) {
                    const provider = card.getAttribute('data-provider');
                    if (provider) {
                        this.showAddModelDialogForProvider(provider);
                    }
                }
            });
        }
    }

    async loadConfig() {
        try {
            this.config = await invoke<AppConfig>('get_config');
            this.updateUI();
        } catch (error) {
            console.error('加载配置失败:', error);
            this.config = {
                storage_path: '',
                theme: 'system',
                models: [],
                reading_model: '',
                analysis_model: '',
                solving_model: '',
                use_paddle_ocr: false,
                mineru_installed: false,
                paddle_ocr_url: '',
                paddle_ocr_token: ''
            };
        }
    }

    private updateUI() {
        if (!this.config) return;

        // 更新存储路径
        const pathInput = document.getElementById('input-storage-path') as HTMLInputElement;
        if (pathInput && this.config.storage_path) {
            pathInput.value = this.config.storage_path;
        }

        // 更新主题选择
        const themeRadios = document.querySelectorAll('input[name="theme"]');
        themeRadios.forEach((radio: Element) => {
            const input = radio as HTMLInputElement;
            if (input.value === this.config?.theme) {
                input.checked = true;
            }
        });

        // 更新模型列表
        this.renderModelList();
        this.updateModelSelects();
        this.updateProviderCards();
        
        // 更新工具设置
        this.updateToolsSettings();
    }

    private async updateToolsSettings() {
        if (!this.config) return;

        // 更新 MinerU 状态
        const mineruBadge = document.getElementById('mineru-status-badge');
        const installBtn = document.getElementById('btn-install-mineru') as HTMLButtonElement;
        
        try {
            const isInstalled = await invoke<boolean>('check_mineru_installed');
            this.config.mineru_installed = isInstalled;
            
            if (mineruBadge) {
                if (isInstalled) {
                    mineruBadge.textContent = '已安装';
                    mineruBadge.classList.add('installed');
                    mineruBadge.classList.remove('installing');
                } else {
                    mineruBadge.textContent = '未安装';
                    mineruBadge.classList.remove('installed', 'installing');
                }
            }
            
            if (installBtn) {
                installBtn.disabled = isInstalled;
                if (isInstalled) {
                    installBtn.innerHTML = '<i class="bi bi-check-circle"></i> 已安装';
                }
            }
        } catch (error) {
            console.error('检测 MinerU 状态失败:', error);
        }

        // 更新 PaddleOCR 设置
        const paddleToggle = document.getElementById('toggle-paddle-ocr') as HTMLInputElement;
        const paddleConfig = document.getElementById('paddle-ocr-config');
        const paddleUrlInput = document.getElementById('input-paddle-url') as HTMLInputElement;
        const paddleTokenInput = document.getElementById('input-paddle-token') as HTMLInputElement;

        if (paddleToggle) {
            paddleToggle.checked = this.config.use_paddle_ocr;
        }
        
        if (paddleConfig) {
            paddleConfig.classList.toggle('visible', this.config.use_paddle_ocr);
        }

        if (paddleUrlInput && this.config.paddle_ocr_url) {
            paddleUrlInput.value = this.config.paddle_ocr_url;
        }

        if (paddleTokenInput && this.config.paddle_ocr_token) {
            paddleTokenInput.value = this.config.paddle_ocr_token;
        }
    }

    private renderModelList() {
        const container = document.getElementById('model-list');
        if (!container || !this.config) return;

        if (this.config.models.length === 0) {
            container.innerHTML = '';
            return;
        }

        container.innerHTML = this.config.models.map(model => {
            const providerConfig = PROVIDER_CONFIG[model.provider] || PROVIDER_CONFIG.custom;
            return `
                <div class="model-item" data-model-id="${model.id}">
                    ${providerConfig.icon ? `<img src="${providerConfig.icon}" alt="${providerConfig.name}" class="model-item-icon">` : '<i class="bi bi-cpu model-item-icon" style="font-size: 24px;"></i>'}
                    <div class="model-item-info">
                        <div class="model-item-name">${model.name}</div>
                        <div class="model-item-provider">${providerConfig.name} - ${model.model_name}</div>
                    </div>
                    <div class="model-item-actions">
                        <button class="btn btn-sm btn-outline-danger btn-delete-model" data-model-id="${model.id}">
                            <i class="bi bi-trash"></i>
                        </button>
                    </div>
                </div>
            `;
        }).join('');

        // 绑定删除按钮事件
        container.querySelectorAll('.btn-delete-model').forEach(btn => {
            btn.addEventListener('click', async (e) => {
                const modelId = (e.currentTarget as HTMLElement).dataset.modelId;
                if (modelId) {
                    await this.removeModel(modelId);
                }
            });
        });
    }

    private updateProviderCards() {
        if (!this.config) return;

        // 获取已配置的供应商
        const configuredProviders = new Set(this.config.models.map(m => m.provider));

        // 更新卡片状态
        document.querySelectorAll('.provider-card').forEach(card => {
            const provider = card.getAttribute('data-provider');
            if (provider && configuredProviders.has(provider)) {
                card.classList.add('configured');
            } else {
                card.classList.remove('configured');
            }
        });
    }

    private updateModelSelects() {
        if (!this.config) return;

        const readingSelect = document.getElementById('select-reading-model') as HTMLSelectElement;
        const analysisSelect = document.getElementById('select-analysis-model') as HTMLSelectElement;
        const solvingSelect = document.getElementById('select-solving-model') as HTMLSelectElement;

        const selects = [readingSelect, analysisSelect, solvingSelect];
        const selectedValues = [
            this.config.reading_model,
            this.config.analysis_model,
            this.config.solving_model
        ];

        selects.forEach((select, index) => {
            if (!select) return;

            select.innerHTML = `<option value="">请选择模型</option>` +
                this.config!.models.map(model => 
                    `<option value="${model.id}" ${selectedValues[index] === model.id ? 'selected' : ''}>
                        ${model.name}
                    </option>`
                ).join('');
        });
    }

    showSettings() {
        const modal = document.getElementById('settings-modal');
        if (modal) {
            modal.style.display = 'flex';
            this.loadConfig(); // 重新加载配置
            this.switchSection('general'); // 默认显示常规设置
        }
    }

    hideSettings() {
        const modal = document.getElementById('settings-modal');
        if (modal) {
            modal.style.display = 'none';
        }
    }

    async saveSettings() {
        if (!this.config) return;

        // 获取表单值
        const readingSelect = document.getElementById('select-reading-model') as HTMLSelectElement;
        const analysisSelect = document.getElementById('select-analysis-model') as HTMLSelectElement;
        const solvingSelect = document.getElementById('select-solving-model') as HTMLSelectElement;

        this.config.reading_model = readingSelect?.value || '';
        this.config.analysis_model = analysisSelect?.value || '';
        this.config.solving_model = solvingSelect?.value || '';

        // 获取主题设置
        const themeRadio = document.querySelector('input[name="theme"]:checked') as HTMLInputElement;
        if (themeRadio) {
            this.config.theme = themeRadio.value;
        }

        // 获取 PaddleOCR 设置
        const paddleToggle = document.getElementById('toggle-paddle-ocr') as HTMLInputElement;
        const paddleUrlInput = document.getElementById('input-paddle-url') as HTMLInputElement;
        const paddleTokenInput = document.getElementById('input-paddle-token') as HTMLInputElement;

        this.config.use_paddle_ocr = paddleToggle?.checked || false;
        this.config.paddle_ocr_url = paddleUrlInput?.value?.trim() || '';
        this.config.paddle_ocr_token = paddleTokenInput?.value?.trim() || '';

        try {
            await invoke('save_config', { configData: this.config });
            
            // 应用主题
            const themeManager = getThemeManager();
            if (themeManager) {
                themeManager.setTheme(this.config.theme as 'light' | 'dark' | 'system');
            }

            this.hideSettings();
        } catch (error) {
            console.error('保存配置失败:', error);
        }
    }

    async browseStoragePath() {
        const selected = await open({
            directory: true,
            multiple: false,
            title: '选择存储路径'
        });

        if (selected && typeof selected === 'string') {
            const pathInput = document.getElementById('input-storage-path') as HTMLInputElement;
            if (pathInput) {
                pathInput.value = selected;
            }
            
            if (this.config) {
                this.config.storage_path = selected;
            }

            // 保存路径
            try {
                await invoke('set_storage_path', { path: selected });
            } catch (error) {
                console.error('设置存储路径失败:', error);
            }
        }
    }

    showAddModelDialogForProvider(provider: string) {
        this.selectedProvider = provider;
        const providerConfig = PROVIDER_CONFIG[provider] || PROVIDER_CONFIG.custom;
        
        const modal = document.getElementById('add-model-modal');
        const titleEl = document.getElementById('add-model-title');
        const iconEl = document.getElementById('modal-provider-icon') as HTMLImageElement;
        const providerSelectGroup = document.getElementById('provider-select-group');
        const providerSelect = document.getElementById('select-model-provider') as HTMLSelectElement;
        const urlInput = document.getElementById('input-model-url') as HTMLInputElement;
        const apiKeyGroup = document.getElementById('api-key-group');
        
        if (modal) {
            modal.style.display = 'flex';
            
            // 设置标题和图标
            if (titleEl) {
                titleEl.textContent = provider === 'custom' ? '添加自定义模型' : `添加 ${providerConfig.name} 模型`;
            }
            
            if (iconEl) {
                if (providerConfig.icon) {
                    iconEl.src = providerConfig.icon;
                    iconEl.style.display = 'block';
                } else {
                    iconEl.style.display = 'none';
                }
            }
            
            // 显示/隐藏供应商选择
            if (providerSelectGroup) {
                providerSelectGroup.style.display = provider === 'custom' ? 'block' : 'none';
            }
            
            // 设置默认值
            if (providerSelect) {
                providerSelect.value = provider;
            }
            
            if (urlInput) {
                urlInput.value = providerConfig.defaultUrl;
                urlInput.placeholder = providerConfig.defaultUrl || '请输入 API URL';
            }
            
            // Ollama 本地模型不需要 API Key
            if (apiKeyGroup) {
                apiKeyGroup.style.display = provider === 'ollama' ? 'none' : 'block';
            }
            
            // 清空其他表单
            const nameInput = document.getElementById('input-model-name') as HTMLInputElement;
            const keyInput = document.getElementById('input-model-key') as HTMLInputElement;
            const modelIdInput = document.getElementById('input-model-id') as HTMLInputElement;
            
            if (nameInput) nameInput.value = '';
            if (keyInput) keyInput.value = '';
            if (modelIdInput) modelIdInput.value = '';
        }
    }

    showAddModelDialog() {
        this.showAddModelDialogForProvider('custom');
    }

    hideAddModelDialog() {
        const modal = document.getElementById('add-model-modal');
        if (modal) {
            modal.style.display = 'none';
        }
        this.selectedProvider = '';
    }

    async addModel() {
        const nameInput = document.getElementById('input-model-name') as HTMLInputElement;
        const providerSelect = document.getElementById('select-model-provider') as HTMLSelectElement;
        const urlInput = document.getElementById('input-model-url') as HTMLInputElement;
        const keyInput = document.getElementById('input-model-key') as HTMLInputElement;
        const modelIdInput = document.getElementById('input-model-id') as HTMLInputElement;

        const name = nameInput?.value?.trim();
        const provider = this.selectedProvider || providerSelect?.value || 'custom';
        const apiUrl = urlInput?.value?.trim();
        const apiKey = keyInput?.value?.trim() || '';
        const modelName = modelIdInput?.value?.trim();

        // Ollama 不需要 API Key
        const needsApiKey = provider !== 'ollama';

        if (!name || !apiUrl || !modelName) {
            alert('请填写所有必填字段');
            return;
        }

        if (needsApiKey && !apiKey) {
            alert('请填写 API Key');
            return;
        }

        const model: ModelConfig = {
            id: `model_${Date.now()}`,
            name,
            provider,
            api_url: apiUrl,
            api_key: apiKey,
            model_name: modelName
        };

        try {
            await invoke('add_model', { model });
            
            if (this.config) {
                this.config.models.push(model);
                this.renderModelList();
                this.updateModelSelects();
                this.updateProviderCards();
            }

            this.hideAddModelDialog();
        } catch (error) {
            console.error('添加模型失败:', error);
        }
    }

    async removeModel(modelId: string) {
        try {
            await invoke('remove_model', { modelId });
            
            if (this.config) {
                this.config.models = this.config.models.filter(m => m.id !== modelId);
                
                // 清除引用
                if (this.config.reading_model === modelId) {
                    this.config.reading_model = '';
                }
                if (this.config.analysis_model === modelId) {
                    this.config.analysis_model = '';
                }
                if (this.config.solving_model === modelId) {
                    this.config.solving_model = '';
                }
                
                this.renderModelList();
                this.updateModelSelects();
                this.updateProviderCards();
            }
        } catch (error) {
            console.error('删除模型失败:', error);
        }
    }

    async testModel() {
        const urlInput = document.getElementById('input-model-url') as HTMLInputElement;
        const keyInput = document.getElementById('input-model-key') as HTMLInputElement;
        const modelIdInput = document.getElementById('input-model-id') as HTMLInputElement;
        const testButton = document.getElementById('btn-test-model') as HTMLButtonElement;

        const apiUrl = urlInput?.value?.trim();
        const apiKey = keyInput?.value?.trim() || '';
        const modelName = modelIdInput?.value?.trim();
        const provider = this.selectedProvider;

        // Ollama 不需要 API Key
        const needsApiKey = provider !== 'ollama';

        if (!apiUrl || !modelName) {
            alert('请先填写 API URL 和模型标识');
            return;
        }

        if (needsApiKey && !apiKey) {
            alert('请先填写 API Key');
            return;
        }

        // 设置按钮为加载状态
        const originalContent = testButton.innerHTML;
        testButton.innerHTML = '<i class="bi bi-arrow-repeat spin"></i> 测试中...';
        testButton.disabled = true;

        try {
            const response = await invoke<string>('test_model', {
                apiUrl,
                apiKey,
                modelName
            });
            
            // 测试成功
            testButton.innerHTML = '<i class="bi bi-check-circle"></i> 连接成功';
            testButton.classList.remove('btn-outline-primary', 'btn-outline-danger');
            testButton.classList.add('btn-outline-success');
            
            console.log('模型响应:', response);
            
            // 3秒后恢复按钮
            setTimeout(() => {
                testButton.innerHTML = originalContent;
                testButton.classList.remove('btn-outline-success');
                testButton.classList.add('btn-outline-primary');
                testButton.disabled = false;
            }, 3000);
        } catch (error) {
            // 测试失败
            testButton.innerHTML = '<i class="bi bi-x-circle"></i> 连接失败';
            testButton.classList.remove('btn-outline-primary', 'btn-outline-success');
            testButton.classList.add('btn-outline-danger');
            
            console.error('测试模型失败:', error);
            alert(`测试失败: ${error}`);
            
            // 3秒后恢复按钮
            setTimeout(() => {
                testButton.innerHTML = originalContent;
                testButton.classList.remove('btn-outline-danger');
                testButton.classList.add('btn-outline-primary');
                testButton.disabled = false;
            }, 3000);
        }
    }

    private mineruInstallUnlisten: UnlistenFn | null = null;

    async installMinerU() {
        const installBtn = document.getElementById('btn-install-mineru') as HTMLButtonElement;
        const badge = document.getElementById('mineru-status-badge');
        const terminalContainer = document.getElementById('terminal-container');
        const terminalOutput = document.getElementById('terminal-output');
        
        if (!installBtn) return;

        const originalContent = installBtn.innerHTML;
        installBtn.innerHTML = '<i class="bi bi-arrow-repeat spin"></i> 安装中...';
        installBtn.disabled = true;
        
        if (badge) {
            badge.textContent = '安装中';
            badge.classList.add('installing');
            badge.classList.remove('installed');
        }

        // 显示终端容器
        if (terminalContainer) {
            terminalContainer.style.display = 'block';
        }
        if (terminalOutput) {
            terminalOutput.innerHTML = '';
        }

        // 监听安装输出事件
        this.mineruInstallUnlisten = await listen<{type: string, message: string}>('mineru-install-output', (event) => {
            if (terminalOutput) {
                this.appendTerminalOutput(terminalOutput, event.payload.type, event.payload.message);
            }
        });

        try {
            await invoke<string>('install_mineru');
            
            installBtn.innerHTML = '<i class="bi bi-check-circle"></i> 已安装';
            
            if (badge) {
                badge.textContent = '已安装';
                badge.classList.remove('installing');
                badge.classList.add('installed');
            }
            
            if (this.config) {
                this.config.mineru_installed = true;
            }
        } catch (error) {
            installBtn.innerHTML = originalContent;
            installBtn.disabled = false;
            
            if (badge) {
                badge.textContent = '安装失败';
                badge.classList.remove('installing', 'installed');
            }
            
            console.error('安装 MinerU 失败:', error);
        } finally {
            // 停止监听
            if (this.mineruInstallUnlisten) {
                this.mineruInstallUnlisten();
                this.mineruInstallUnlisten = null;
            }
        }
    }

    closeTerminal() {
        const terminalContainer = document.getElementById('terminal-container');
        if (terminalContainer) {
            terminalContainer.style.display = 'none';
        }
    }

    async checkMinerU() {
        const badge = document.getElementById('mineru-status-badge');
        const installBtn = document.getElementById('btn-install-mineru') as HTMLButtonElement;
        const checkBtn = document.getElementById('btn-check-mineru') as HTMLButtonElement;
        const pathInfo = document.getElementById('mineru-path-info');
        const exePath = document.getElementById('mineru-exe-path');
        const pathStatus = document.getElementById('mineru-path-status');

        if (checkBtn) {
            checkBtn.innerHTML = '<i class="bi bi-arrow-repeat spin"></i>';
            checkBtn.disabled = true;
        }

        try {
            // 使用新的详情 API（包含模型状态）
            const info = await invoke<MineruInstallInfo>('get_mineru_full_info');
            
            if (this.config) {
                this.config.mineru_installed = info.is_installed;
            }
            
            // 更新状态徽章
            if (badge) {
                if (info.command_available && info.models_downloaded) {
                    badge.textContent = '已就绪';
                    badge.classList.add('installed');
                    badge.classList.remove('installing');
                } else if (info.is_installed) {
                    badge.textContent = info.models_downloaded ? '已就绪' : '需下载模型';
                    badge.classList.remove('installed');
                    badge.classList.add('installing');
                } else {
                    badge.textContent = '未安装';
                    badge.classList.remove('installed', 'installing');
                }
            }
            
            // 更新路径信息
            if (pathInfo) {
                if (info.is_installed) {
                    pathInfo.style.display = 'block';
                    
                    if (exePath) {
                        exePath.textContent = info.executable_path || '未找到可执行文件';
                    }
                    
                    if (pathStatus) {
                        if (info.command_available) {
                            pathStatus.innerHTML = '<i class="bi bi-check-circle-fill text-success"></i><span>命令可用</span>';
                        } else {
                            pathStatus.innerHTML = '<i class="bi bi-exclamation-triangle-fill text-warning"></i><span>命令不可用，请检查 Python Scripts 目录是否在 PATH 中</span>';
                        }
                    }
                } else {
                    pathInfo.style.display = 'none';
                }
            }
            
            // 更新安装按钮
            if (installBtn) {
                installBtn.disabled = info.command_available;
                if (info.command_available) {
                    installBtn.innerHTML = '<i class="bi bi-check-circle"></i> 已安装';
                } else if (info.is_installed) {
                    installBtn.innerHTML = '<i class="bi bi-arrow-repeat"></i> 重新检测路径';
                } else {
                    installBtn.innerHTML = '<i class="bi bi-download"></i> 安装 MinerU';
                }
            }

            // 更新模型状态
            this.updateModelStatus(info);
        } catch (error) {
            console.error('检测 MinerU 失败:', error);
        } finally {
            if (checkBtn) {
                checkBtn.innerHTML = '<i class="bi bi-arrow-repeat"></i> 检测';
                checkBtn.disabled = false;
            }
        }
    }

    private updateModelStatus(info: MineruInstallInfo) {
        // 更新模型目录显示 - 始终显示路径（即使未下载）
        const modelsDir = document.getElementById('mineru-models-dir');
        if (modelsDir) {
            if (info.models_dir) {
                modelsDir.textContent = info.models_dir;
            } else {
                modelsDir.textContent = '请先配置 BooQ 存储路径';
            }
        }

        // 更新 ModelScope 状态
        const modelscopeBadge = document.getElementById('modelscope-badge');
        const btnInstallModelscope = document.getElementById('btn-install-modelscope') as HTMLButtonElement;
        
        if (modelscopeBadge) {
            if (info.modelscope_installed) {
                modelscopeBadge.textContent = '已安装';
                modelscopeBadge.classList.add('installed');
                modelscopeBadge.classList.remove('installing');
            } else {
                modelscopeBadge.textContent = '未安装';
                modelscopeBadge.classList.remove('installed', 'installing');
            }
        }
        
        if (btnInstallModelscope) {
            btnInstallModelscope.disabled = info.modelscope_installed;
            if (info.modelscope_installed) {
                btnInstallModelscope.innerHTML = '<i class="bi bi-check-circle"></i> 已安装';
            } else {
                btnInstallModelscope.innerHTML = '<i class="bi bi-download"></i> 安装 ModelScope';
            }
        }

        // 更新主模型状态
        const mainModelBadge = document.getElementById('main-model-badge');
        const btnDownloadMainModel = document.getElementById('btn-download-main-model') as HTMLButtonElement;
        
        if (mainModelBadge) {
            if (info.models_downloaded) {
                mainModelBadge.textContent = '已下载';
                mainModelBadge.classList.add('installed');
                mainModelBadge.classList.remove('installing');
            } else {
                mainModelBadge.textContent = '未下载';
                mainModelBadge.classList.remove('installed', 'installing');
            }
        }
        
        if (btnDownloadMainModel) {
            btnDownloadMainModel.disabled = info.models_downloaded;
            if (info.models_downloaded) {
                btnDownloadMainModel.innerHTML = '<i class="bi bi-check-circle"></i> 已下载';
            } else {
                btnDownloadMainModel.innerHTML = '<i class="bi bi-download"></i> 下载主模型';
            }
        }

        // 更新 OCR 模型状态
        const ocrModelBadge = document.getElementById('ocr-model-badge');
        const btnDownloadOcrModel = document.getElementById('btn-download-ocr-model') as HTMLButtonElement;
        
        if (ocrModelBadge) {
            if (info.ocr_models_downloaded) {
                ocrModelBadge.textContent = '已下载';
                ocrModelBadge.classList.add('installed');
                ocrModelBadge.classList.remove('installing');
            } else {
                ocrModelBadge.textContent = '未下载';
                ocrModelBadge.classList.remove('installed', 'installing');
            }
        }
        
        if (btnDownloadOcrModel) {
            btnDownloadOcrModel.disabled = info.ocr_models_downloaded;
            if (info.ocr_models_downloaded) {
                btnDownloadOcrModel.innerHTML = '<i class="bi bi-check-circle"></i> 已下载';
            } else {
                btnDownloadOcrModel.innerHTML = '<i class="bi bi-download"></i> 下载 OCR 模型';
            }
        }

        // 更新配置按钮状态
        const btnUpdateConfig = document.getElementById('btn-update-mineru-config') as HTMLButtonElement;
        if (btnUpdateConfig) {
            btnUpdateConfig.disabled = !info.models_downloaded;
        }
    }

    // 终端输出缓冲区（用于批量处理输出）
    private terminalBuffers: Map<string, { messages: Array<{type: string, message: string}>, pending: boolean }> = new Map();

    // 批量追加终端输出（解决输出刷新太快的问题）
    private appendTerminalOutput(terminalOutput: HTMLElement, type: string, message: string) {
        const terminalId = terminalOutput.id;
        
        if (!this.terminalBuffers.has(terminalId)) {
            this.terminalBuffers.set(terminalId, { messages: [], pending: false });
        }
        
        const buffer = this.terminalBuffers.get(terminalId)!;
        buffer.messages.push({ type, message });
        
        // 如果没有待处理的动画帧，安排一次更新
        if (!buffer.pending) {
            buffer.pending = true;
            requestAnimationFrame(() => {
                this.flushTerminalBuffer(terminalOutput, terminalId);
            });
        }
    }

    private flushTerminalBuffer(terminalOutput: HTMLElement, terminalId: string) {
        const buffer = this.terminalBuffers.get(terminalId);
        if (!buffer || buffer.messages.length === 0) return;
        
        // 创建文档片段以批量插入
        const fragment = document.createDocumentFragment();
        
        for (const { type, message } of buffer.messages) {
            const span = document.createElement('span');
            span.className = `${type}-line`;
            span.textContent = message;
            fragment.appendChild(span);
        }
        
        terminalOutput.appendChild(fragment);
        terminalOutput.scrollTop = terminalOutput.scrollHeight;
        
        // 清空缓冲区
        buffer.messages = [];
        buffer.pending = false;
    }

    private modelscopeUnlisten: UnlistenFn | null = null;
    private mainModelUnlisten: UnlistenFn | null = null;
    private ocrModelUnlisten: UnlistenFn | null = null;

    async installModelScope() {
        const btn = document.getElementById('btn-install-modelscope') as HTMLButtonElement;
        const badge = document.getElementById('modelscope-badge');
        const terminalContainer = document.getElementById('modelscope-terminal-container');
        const terminalOutput = document.getElementById('modelscope-terminal-output');
        
        if (!btn) return;

        const originalContent = btn.innerHTML;
        btn.innerHTML = '<i class="bi bi-arrow-repeat spin"></i> 安装中...';
        btn.disabled = true;

        if (badge) {
            badge.textContent = '安装中';
            badge.classList.add('installing');
            badge.classList.remove('installed');
        }

        // 显示终端容器
        if (terminalContainer) {
            terminalContainer.style.display = 'block';
        }
        if (terminalOutput) {
            terminalOutput.innerHTML = '';
        }

        // 监听输出事件
        this.modelscopeUnlisten = await listen<{type: string, model_type: string, message: string}>('mineru-model-output', (event) => {
            if (terminalOutput && event.payload.model_type === 'deps') {
                this.appendTerminalOutput(terminalOutput, event.payload.type, event.payload.message);
            }
        });

        try {
            await invoke<string>('install_modelscope');
            btn.innerHTML = '<i class="bi bi-check-circle"></i> 已安装';
            
            if (badge) {
                badge.textContent = '已安装';
                badge.classList.remove('installing');
                badge.classList.add('installed');
            }

            // 刷新状态
            await this.checkMinerU();
        } catch (error) {
            btn.innerHTML = originalContent;
            btn.disabled = false;
            
            if (badge) {
                badge.textContent = '安装失败';
                badge.classList.remove('installing', 'installed');
            }
            
            console.error('安装 modelscope 失败:', error);
        } finally {
            if (this.modelscopeUnlisten) {
                this.modelscopeUnlisten();
                this.modelscopeUnlisten = null;
            }
        }
    }

    async downloadMainModel() {
        const btn = document.getElementById('btn-download-main-model') as HTMLButtonElement;
        const badge = document.getElementById('main-model-badge');
        const terminalContainer = document.getElementById('main-model-terminal-container');
        const terminalOutput = document.getElementById('main-model-terminal-output');
        
        if (!btn) return;

        const originalContent = btn.innerHTML;
        btn.innerHTML = '<i class="bi bi-arrow-repeat spin"></i> 下载中...';
        btn.disabled = true;

        if (badge) {
            badge.textContent = '下载中';
            badge.classList.add('installing');
            badge.classList.remove('installed');
        }

        // 显示终端容器
        if (terminalContainer) {
            terminalContainer.style.display = 'block';
        }
        if (terminalOutput) {
            terminalOutput.innerHTML = '';
        }

        // 监听输出事件
        this.mainModelUnlisten = await listen<{type: string, model_type: string, message: string}>('mineru-model-output', (event) => {
            if (terminalOutput && event.payload.model_type === 'main') {
                this.appendTerminalOutput(terminalOutput, event.payload.type, event.payload.message);
            }
        });

        try {
            await invoke<string>('download_mineru_models');
            btn.innerHTML = '<i class="bi bi-check-circle"></i> 已下载';
            
            if (badge) {
                badge.textContent = '已下载';
                badge.classList.remove('installing');
                badge.classList.add('installed');
            }

            // 刷新状态
            await this.checkMinerU();
        } catch (error) {
            btn.innerHTML = originalContent;
            btn.disabled = false;
            
            if (badge) {
                badge.textContent = '下载失败';
                badge.classList.remove('installing', 'installed');
            }
            
            console.error('下载主模型失败:', error);
        } finally {
            if (this.mainModelUnlisten) {
                this.mainModelUnlisten();
                this.mainModelUnlisten = null;
            }
        }
    }

    async downloadOcrModel() {
        const btn = document.getElementById('btn-download-ocr-model') as HTMLButtonElement;
        const badge = document.getElementById('ocr-model-badge');
        const terminalContainer = document.getElementById('ocr-model-terminal-container');
        const terminalOutput = document.getElementById('ocr-model-terminal-output');
        
        if (!btn) return;

        const originalContent = btn.innerHTML;
        btn.innerHTML = '<i class="bi bi-arrow-repeat spin"></i> 下载中...';
        btn.disabled = true;

        if (badge) {
            badge.textContent = '下载中';
            badge.classList.add('installing');
            badge.classList.remove('installed');
        }

        // 显示终端容器
        if (terminalContainer) {
            terminalContainer.style.display = 'block';
        }
        if (terminalOutput) {
            terminalOutput.innerHTML = '';
        }

        // 监听输出事件
        this.ocrModelUnlisten = await listen<{type: string, model_type: string, message: string}>('mineru-model-output', (event) => {
            if (terminalOutput && event.payload.model_type === 'ocr') {
                this.appendTerminalOutput(terminalOutput, event.payload.type, event.payload.message);
            }
        });

        try {
            await invoke<string>('download_ocr_models');
            btn.innerHTML = '<i class="bi bi-check-circle"></i> 已下载';
            
            if (badge) {
                badge.textContent = '已下载';
                badge.classList.remove('installing');
                badge.classList.add('installed');
            }

            // 刷新状态
            await this.checkMinerU();
        } catch (error) {
            btn.innerHTML = originalContent;
            btn.disabled = false;
            
            if (badge) {
                badge.textContent = '下载失败';
                badge.classList.remove('installing', 'installed');
            }
            
            console.error('下载 OCR 模型失败:', error);
        } finally {
            if (this.ocrModelUnlisten) {
                this.ocrModelUnlisten();
                this.ocrModelUnlisten = null;
            }
        }
    }

    closeModelscopeTerminal() {
        const terminalContainer = document.getElementById('modelscope-terminal-container');
        if (terminalContainer) {
            terminalContainer.style.display = 'none';
        }
    }

    closeMainModelTerminal() {
        const terminalContainer = document.getElementById('main-model-terminal-container');
        if (terminalContainer) {
            terminalContainer.style.display = 'none';
        }
    }

    closeOcrModelTerminal() {
        const terminalContainer = document.getElementById('ocr-model-terminal-container');
        if (terminalContainer) {
            terminalContainer.style.display = 'none';
        }
    }

    async updateMineruConfig() {
        const btn = document.getElementById('btn-update-mineru-config') as HTMLButtonElement;
        
        if (!btn) return;

        const originalContent = btn.innerHTML;
        btn.innerHTML = '<i class="bi bi-arrow-repeat spin"></i> 更新中...';
        btn.disabled = true;

        try {
            await invoke<string>('update_mineru_config');
            btn.innerHTML = '<i class="bi bi-check-circle"></i> 配置成功';
            
            // 3秒后恢复按钮
            setTimeout(() => {
                btn.innerHTML = originalContent;
                btn.disabled = false;
            }, 3000);
            
            // 刷新状态
            await this.checkMinerU();
        } catch (error) {
            btn.innerHTML = originalContent;
            btn.disabled = false;
            console.error('更新配置失败:', error);
            alert('更新配置失败: ' + error);
        }
    }

    togglePaddleOcrConfig() {
        const paddleToggle = document.getElementById('toggle-paddle-ocr') as HTMLInputElement;
        const paddleConfig = document.getElementById('paddle-ocr-config');

        if (paddleConfig && paddleToggle) {
            paddleConfig.classList.toggle('visible', paddleToggle.checked);
        }
    }

    // ==================== 日志管理 ====================

    async loadLogs() {
        try {
            const logs = await invoke<LogEntry[]>('get_logs');
            this.displayLogs(logs);
        } catch (error) {
            console.error('加载日志失败:', error);
        }
    }

    private displayLogs(logs: LogEntry[]) {
        const container = document.getElementById('log-container');
        if (!container) return;

        // 获取过滤条件
        const filters = this.getLogFilters();

        // 过滤日志
        const filteredLogs = logs.filter(log => filters.includes(log.level.toLowerCase()));

        if (filteredLogs.length === 0) {
            container.innerHTML = '<div class="log-empty">暂无日志记录</div>';
            return;
        }

        // 渲染日志条目（按时间倒序）
        const html = filteredLogs.reverse().map(log => `
            <div class="log-entry">
                <span class="log-time">${log.timestamp}</span>
                <span class="log-level level-${log.level.toLowerCase()}">${log.level}</span>
                <span class="log-source">${log.source}</span>
                <span class="log-message">${this.escapeHtml(log.message)}</span>
            </div>
        `).join('');

        container.innerHTML = html;
    }

    private getLogFilters(): string[] {
        const filters: string[] = [];
        document.querySelectorAll('.log-filter-check').forEach((checkbox) => {
            const input = checkbox as HTMLInputElement;
            if (input.checked) {
                const level = input.getAttribute('data-level');
                if (level) filters.push(level);
            }
        });
        return filters;
    }

    private escapeHtml(text: string): string {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    async clearLogs() {
        try {
            await invoke('clear_logs');
            const container = document.getElementById('log-container');
            if (container) {
                container.innerHTML = '<div class="log-empty">暂无日志记录</div>';
            }
        } catch (error) {
            console.error('清空日志失败:', error);
        }
    }

    bindLogEvents() {
        // 刷新日志按钮
        const refreshBtn = document.getElementById('btn-refresh-logs');
        if (refreshBtn) {
            refreshBtn.addEventListener('click', () => this.loadLogs());
        }

        // 清空日志按钮
        const clearBtn = document.getElementById('btn-clear-logs');
        if (clearBtn) {
            clearBtn.addEventListener('click', () => this.clearLogs());
        }

        // 过滤器变化时刷新
        document.querySelectorAll('.log-filter-check').forEach((checkbox) => {
            checkbox.addEventListener('change', () => this.loadLogs());
        });
    }

    getConfig(): AppConfig | null {
        return this.config;
    }
}
