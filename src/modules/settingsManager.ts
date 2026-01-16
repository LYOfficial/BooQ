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

export class SettingsManager {
    private config: AppConfig | null = null;

    async init() {
        await this.loadConfig();
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
    }

    private renderModelList() {
        const container = document.getElementById('model-list');
        if (!container || !this.config) return;

        if (this.config.models.length === 0) {
            container.innerHTML = `
                <div class="empty-state" style="padding: 20px;">
                    <p class="text-muted">暂未添加模型</p>
                </div>
            `;
            return;
        }

        container.innerHTML = this.config.models.map(model => `
            <div class="model-item" data-model-id="${model.id}">
                <div class="model-item-info">
                    <div class="model-item-name">${model.name}</div>
                    <div class="model-item-provider">${this.getProviderName(model.provider)} - ${model.model_name}</div>
                </div>
                <div class="model-item-actions">
                    <button class="btn btn-sm btn-outline-danger btn-delete-model" data-model-id="${model.id}">
                        <i class="bi bi-trash"></i>
                    </button>
                </div>
            </div>
        `).join('');

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

    private getProviderName(provider: string): string {
        const names: Record<string, string> = {
            'openai': 'OpenAI',
            'azure': 'Azure OpenAI',
            'anthropic': 'Anthropic',
            'deepseek': 'DeepSeek',
            'zhipu': '智谱AI',
            'qwen': '通义千问',
            'custom': '自定义'
        };
        return names[provider] || provider;
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

    showAddModelDialog() {
        const modal = document.getElementById('add-model-modal');
        if (modal) {
            modal.style.display = 'flex';
            
            // 清空表单
            const inputs = ['input-model-name', 'input-model-url', 'input-model-key', 'input-model-id'];
            inputs.forEach(id => {
                const input = document.getElementById(id) as HTMLInputElement;
                if (input) input.value = '';
            });
            
            const providerSelect = document.getElementById('select-model-provider') as HTMLSelectElement;
            if (providerSelect) providerSelect.value = 'openai';
        }
    }

    hideAddModelDialog() {
        const modal = document.getElementById('add-model-modal');
        if (modal) {
            modal.style.display = 'none';
        }
    }

    async addModel() {
        const nameInput = document.getElementById('input-model-name') as HTMLInputElement;
        const providerSelect = document.getElementById('select-model-provider') as HTMLSelectElement;
        const urlInput = document.getElementById('input-model-url') as HTMLInputElement;
        const keyInput = document.getElementById('input-model-key') as HTMLInputElement;
        const modelIdInput = document.getElementById('input-model-id') as HTMLInputElement;

        const name = nameInput?.value?.trim();
        const provider = providerSelect?.value;
        const apiUrl = urlInput?.value?.trim();
        const apiKey = keyInput?.value?.trim();
        const modelName = modelIdInput?.value?.trim();

        if (!name || !apiUrl || !apiKey || !modelName) {
            alert('请填写所有必填字段');
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
            }
        } catch (error) {
            console.error('删除模型失败:', error);
        }
    }

    getConfig(): AppConfig | null {
        return this.config;
    }
}
