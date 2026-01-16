// 设置管理模块

import { invoke } from '@tauri-apps/api/tauri';
import { open } from '@tauri-apps/api/dialog';
import { getThemeManager } from '../main';

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
                solving_model: ''
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

    getConfig(): AppConfig | null {
        return this.config;
    }
}
