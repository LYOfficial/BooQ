// 主题管理模块

import { invoke } from '@tauri-apps/api/tauri';

export type ThemeMode = 'light' | 'dark' | 'system';

export class ThemeManager {
    private currentTheme: ThemeMode = 'system';
    private actualTheme: 'light' | 'dark' = 'light';
    private mediaQuery: MediaQueryList;

    constructor() {
        this.mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    }

    async init() {
        // 监听系统主题变化
        this.mediaQuery.addEventListener('change', (e) => {
            if (this.currentTheme === 'system') {
                this.applyTheme(e.matches ? 'dark' : 'light');
            }
        });

        // 加载保存的主题设置
        await this.loadSavedTheme();
    }

    private async loadSavedTheme() {
        try {
            const config = await invoke<{ theme: string }>('get_config');
            if (config && config.theme) {
                this.setTheme(config.theme as ThemeMode);
            } else {
                this.setTheme('system');
            }
        } catch (error) {
            console.error('加载主题设置失败:', error);
            this.setTheme('system');
        }
    }

    setTheme(mode: ThemeMode) {
        this.currentTheme = mode;

        if (mode === 'system') {
            // 跟随系统
            const systemDark = this.mediaQuery.matches;
            this.applyTheme(systemDark ? 'dark' : 'light');
        } else {
            this.applyTheme(mode);
        }

        // 更新主题切换按钮图标
        this.updateThemeButton();
    }

    private applyTheme(theme: 'light' | 'dark') {
        this.actualTheme = theme;
        document.documentElement.setAttribute('data-theme', theme);
    }

    cycleTheme() {
        const themes: ThemeMode[] = ['light', 'dark', 'system'];
        const currentIndex = themes.indexOf(this.currentTheme);
        const nextIndex = (currentIndex + 1) % themes.length;
        this.setTheme(themes[nextIndex]);

        // 更新设置中的选项
        const themeRadios = document.querySelectorAll('input[name="theme"]');
        themeRadios.forEach((radio: Element) => {
            const input = radio as HTMLInputElement;
            input.checked = input.value === this.currentTheme;
        });
    }

    private updateThemeButton() {
        const btn = document.getElementById('btn-theme');
        if (!btn) return;

        const icons: Record<ThemeMode, string> = {
            'light': 'bi-sun',
            'dark': 'bi-moon',
            'system': 'bi-circle-half'
        };

        const titles: Record<ThemeMode, string> = {
            'light': '白天模式',
            'dark': '夜间模式',
            'system': '跟随系统'
        };

        const icon = btn.querySelector('i');
        if (icon) {
            icon.className = `bi ${icons[this.currentTheme]}`;
        }
        btn.title = titles[this.currentTheme];
    }

    getCurrentTheme(): ThemeMode {
        return this.currentTheme;
    }

    getActualTheme(): 'light' | 'dark' {
        return this.actualTheme;
    }
}
