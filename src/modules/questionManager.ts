// 题目管理模块

import { invoke } from '@tauri-apps/api/tauri';

// 引入 marked 和 katex
declare const marked: any;
declare const renderMathInElement: any;

export interface Question {
    id: string;
    file_id: string;
    question_type: string;
    chapter: string;
    section: string;
    knowledge_points: string[];
    question_text: string;
    answer: string;
    analysis: string;
    page_number: number;
    has_original_answer: boolean;
}

export interface AnalysisProgress {
    file_id: string;
    status: string;
    current_page: number;
    total_pages: number;
    current_step: string;
    questions_found: number;
    message: string;
}

export class QuestionManager {
    private questions: Question[] = [];
    private currentIndex: number = 0;
    private currentFileId: string | null = null;
    private analysisTimer: number | null = null;
    private showingAnswer: boolean = false;

    async init() {
        // 初始化
    }

    async loadQuestions(fileId: string) {
        this.currentFileId = fileId;
        
        try {
            this.questions = await invoke<Question[]>('get_questions', { fileId });
            this.currentIndex = 0;
            this.renderQuestions();
        } catch (error) {
            console.error('加载题目失败:', error);
            this.questions = [];
            this.renderEmptyState();
        }
    }

    async startAnalysis(fileId: string) {
        this.currentFileId = fileId;

        try {
            await invoke('start_analysis', { fileId });
            
            // 显示进度条
            this.showProgress();
            
            // 开始轮询进度
            this.startProgressPolling();
        } catch (error) {
            console.error('开始分析失败:', error);
        }
    }

    async stopAnalysis() {
        if (!this.currentFileId) return;

        try {
            await invoke('stop_analysis', { fileId: this.currentFileId });
            this.stopProgressPolling();
        } catch (error) {
            console.error('停止分析失败:', error);
        }
    }

    private startProgressPolling() {
        this.stopProgressPolling();
        
        this.analysisTimer = window.setInterval(async () => {
            if (!this.currentFileId) return;

            try {
                const progress = await invoke<AnalysisProgress>('get_analysis_progress', {
                    fileId: this.currentFileId
                });

                this.updateProgress(progress);

                // 如果完成或出错，停止轮询
                if (progress.status === 'completed' || progress.status === 'error' || progress.status === 'stopped') {
                    this.stopProgressPolling();
                    await this.loadQuestions(this.currentFileId);
                }
            } catch (error) {
                console.error('获取进度失败:', error);
            }
        }, 1000);
    }

    private stopProgressPolling() {
        if (this.analysisTimer !== null) {
            window.clearInterval(this.analysisTimer);
            this.analysisTimer = null;
        }
    }

    private showProgress() {
        const progressDiv = document.getElementById('analysis-progress');
        if (progressDiv) {
            progressDiv.style.display = 'block';
        }
    }

    private hideProgress() {
        const progressDiv = document.getElementById('analysis-progress');
        if (progressDiv) {
            progressDiv.style.display = 'none';
        }
    }

    private updateProgress(progress: AnalysisProgress) {
        const progressBar = document.querySelector('.progress-bar') as HTMLElement;
        const progressText = document.getElementById('progress-text');

        if (progressBar && progress.total_pages > 0) {
            const percent = (progress.current_page / progress.total_pages) * 100;
            progressBar.style.width = `${percent}%`;
        }

        if (progressText) {
            progressText.textContent = `${progress.message} (已发现 ${progress.questions_found} 道题目)`;
        }

        // 更新题目数量
        const countSpan = document.getElementById('question-count');
        if (countSpan) {
            countSpan.textContent = `${progress.questions_found} 题`;
        }
    }

    private renderQuestions() {
        const container = document.getElementById('question-panel');
        const navigation = document.getElementById('question-navigation');
        const countSpan = document.getElementById('question-count');

        if (this.questions.length === 0) {
            this.renderEmptyState();
            if (navigation) navigation.style.display = 'none';
            return;
        }

        // 显示导航
        if (navigation) navigation.style.display = 'flex';
        
        // 更新计数
        if (countSpan) {
            countSpan.textContent = `${this.questions.length} 题`;
        }

        // 渲染当前题目
        this.renderCurrentQuestion();
        this.updateQuestionNavigation();
    }

    private renderCurrentQuestion() {
        const container = document.getElementById('question-panel');
        if (!container || this.questions.length === 0) return;

        const question = this.questions[this.currentIndex];
        this.showingAnswer = false;

        container.innerHTML = `
            <div class="question-card">
                <div class="question-meta">
                    <span class="question-tag type-${question.question_type}">
                        ${question.question_type === 'example' ? '例题' : '习题'}
                    </span>
                    ${question.chapter ? `<span class="question-tag">${question.chapter}</span>` : ''}
                    ${question.section ? `<span class="question-tag">${question.section}</span>` : ''}
                    <span class="question-tag">第 ${question.page_number} 页</span>
                </div>
                
                <div class="question-content" id="question-text">
                    ${marked.parse(question.question_text)}
                </div>
                
                ${question.knowledge_points.length > 0 ? `
                    <div class="question-meta">
                        <strong>知识点：</strong>
                        ${question.knowledge_points.map(kp => 
                            `<span class="question-tag">${kp}</span>`
                        ).join('')}
                    </div>
                ` : ''}
                
                <button class="btn btn-outline-success btn-show-answer" id="btn-show-answer">
                    <i class="bi bi-eye"></i> 显示解析
                </button>
                
                <div class="question-answer" id="question-answer" style="display: none;">
                    <div class="question-answer-header">
                        <i class="bi bi-check-circle"></i> 答案
                    </div>
                    <div id="answer-text">${marked.parse(question.answer)}</div>
                </div>
                
                ${question.analysis ? `
                    <div class="question-analysis" id="question-analysis" style="display: none;">
                        <div class="question-analysis-header">
                            <i class="bi bi-lightbulb"></i> 详细解析
                        </div>
                        <div id="analysis-text">${marked.parse(question.analysis)}</div>
                    </div>
                ` : ''}
            </div>
        `;

        // 绑定显示答案按钮
        const showAnswerBtn = document.getElementById('btn-show-answer');
        showAnswerBtn?.addEventListener('click', () => {
            this.toggleAnswer();
        });

        // 渲染数学公式
        this.renderMath(container);
    }

    private toggleAnswer() {
        const answerDiv = document.getElementById('question-answer');
        const analysisDiv = document.getElementById('question-analysis');
        const btn = document.getElementById('btn-show-answer');

        if (this.showingAnswer) {
            if (answerDiv) answerDiv.style.display = 'none';
            if (analysisDiv) analysisDiv.style.display = 'none';
            if (btn) {
                btn.innerHTML = '<i class="bi bi-eye"></i> 显示解析';
            }
        } else {
            if (answerDiv) answerDiv.style.display = 'block';
            if (analysisDiv) analysisDiv.style.display = 'block';
            if (btn) {
                btn.innerHTML = '<i class="bi bi-eye-slash"></i> 隐藏解析';
            }
        }

        this.showingAnswer = !this.showingAnswer;
    }

    private renderMath(container: HTMLElement) {
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

    private updateQuestionNavigation() {
        const indexSpan = document.getElementById('question-index');
        const prevBtn = document.getElementById('btn-prev-question') as HTMLButtonElement;
        const nextBtn = document.getElementById('btn-next-question') as HTMLButtonElement;

        if (indexSpan) {
            indexSpan.textContent = `${this.currentIndex + 1} / ${this.questions.length}`;
        }

        if (prevBtn) {
            prevBtn.disabled = this.currentIndex <= 0;
        }

        if (nextBtn) {
            nextBtn.disabled = this.currentIndex >= this.questions.length - 1;
        }
    }

    private renderEmptyState() {
        const container = document.getElementById('question-panel');
        if (container) {
            container.innerHTML = `
                <div class="empty-state">
                    <i class="bi bi-journal-text"></i>
                    <p>暂无题目</p>
                    <p class="text-muted">上传文件并开始分析后，题目将在这里展示</p>
                </div>
            `;
        }
    }

    prevQuestion() {
        if (this.currentIndex > 0) {
            this.currentIndex--;
            this.renderCurrentQuestion();
            this.updateQuestionNavigation();
        }
    }

    nextQuestion() {
        if (this.currentIndex < this.questions.length - 1) {
            this.currentIndex++;
            this.renderCurrentQuestion();
            this.updateQuestionNavigation();
        }
    }

    getQuestions(): Question[] {
        return this.questions;
    }

    getCurrentQuestion(): Question | null {
        return this.questions[this.currentIndex] || null;
    }
}
