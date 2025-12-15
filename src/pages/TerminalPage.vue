<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

interface Command {
    id: number;
    command: string;
    output: string;
    isRunning: boolean;
}

interface Project {
    id: number;
    name: string;
    path: string;
    commands: Command[];
    isRunning: boolean;
}

interface CommandOutputEvent {
    command_id: string;
    data: string;
    stream: string;
}

interface CommandCompleteEvent {
    command_id: string;
    success: boolean;
}

const projects = ref<Project[]>([]);
let outputUnlisten: UnlistenFn | null = null;
let completeUnlisten: UnlistenFn | null = null;

// Функция для автопрокрутки textarea
function scrollToBottom(commandId: string) {
    setTimeout(() => {
        const textarea = document.querySelector(`[data-command-id="${commandId}"]`) as HTMLTextAreaElement;
        if (textarea) {
            textarea.scrollTop = textarea.scrollHeight;
        }
    }, 0);
}

onMounted(async () => {
    loadProjects();

    // Подписываемся на события вывода команд
    outputUnlisten = await listen<CommandOutputEvent>('command-output', (event) => {
        const { command_id, data, stream } = event.payload;

        // Находим команду по ID
        for (const project of projects.value) {
            const cmd = project.commands.find(c => `${project.id}-${c.id}` === command_id);
            if (cmd) {
                if (stream === 'stderr') {
                    cmd.output += data;
                } else {
                    cmd.output += data;
                }
                // Автопрокрутка вниз
                scrollToBottom(command_id);
                break;
            }
        }
    });

    // Подписываемся на события завершения команд
    completeUnlisten = await listen<CommandCompleteEvent>('command-complete', (event) => {
        const { command_id, success } = event.payload;

        // Находим команду по ID
        for (const project of projects.value) {
            const cmd = project.commands.find(c => `${project.id}-${c.id}` === command_id);
            if (cmd) {
                cmd.isRunning = false;
                if (!success) {
                    cmd.output += '\n--- COMMAND FAILED ---\n';
                }
                break;
            }
        }
    });
});

onUnmounted(() => {
    if (outputUnlisten) outputUnlisten();
    if (completeUnlisten) completeUnlisten();
});

function loadProjects() {
    const saved = localStorage.getItem('terminal_projects');
    if (saved) {
        projects.value = JSON.parse(saved).map((p: Project) => ({
            ...p,
            isRunning: false,
            commands: p.commands.map((c: Command) => ({
                ...c,
                output: '',
                isRunning: false
            }))
        }));
    }
}

function saveProjects() {
    const toSave = projects.value.map(p => ({
        id: p.id,
        name: p.name,
        path: p.path,
        commands: p.commands.map(c => ({
            id: c.id,
            command: c.command,
            output: '',
            isRunning: false
        }))
    }));
    localStorage.setItem('terminal_projects', JSON.stringify(toSave));
}

function addProject() {
    const newProject: Project = {
        id: Date.now(),
        name: 'Новый проект',
        path: '',
        commands: [],
        isRunning: false
    };
    projects.value.push(newProject);
    saveProjects();
}

function deleteProject(id: number) {
    projects.value = projects.value.filter(p => p.id !== id);
    saveProjects();
}

function updateProjectName(id: number, name: string) {
    const project = projects.value.find(p => p.id === id);
    if (project) {
        project.name = name;
        saveProjects();
    }
}

function updateProjectPath(id: number, path: string) {
    const project = projects.value.find(p => p.id === id);
    if (project) {
        project.path = path;
        saveProjects();
    }
}

function addCommand(projectId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (project && project.commands.length < 6) {
        const newCommand: Command = {
            id: Date.now(),
            command: '',
            output: '',
            isRunning: false
        };
        project.commands.push(newCommand);
        saveProjects();
    }
}

function deleteCommand(projectId: number, commandId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
        project.commands = project.commands.filter(c => c.id !== commandId);
        saveProjects();
    }
}

function updateCommand(projectId: number, commandId: number, command: string) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
        const cmd = project.commands.find(c => c.id === commandId);
        if (cmd) {
            cmd.command = command;
            saveProjects();
        }
    }
}

async function runProject(projectId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (!project || project.commands.length === 0) {
        return;
    }

    project.isRunning = true;

    // Очищаем вывод всех команд
    project.commands.forEach(cmd => {
        cmd.output = '';
        cmd.isRunning = true;
    });

    // Запускаем все команды параллельно с потоковым выводом
    const promises = project.commands.map(async (cmd) => {
        const commandId = `${project.id}-${cmd.id}`;
        try {
            await invoke('execute_command_stream', {
                command: cmd.command,
                commandId: commandId,
                workingDirectory: project.path || null
            });
        } catch (error) {
            cmd.output += '\n\nОшибка: ' + error;
            cmd.isRunning = false;
        }
    });

    // Не ждём завершения команд - они будут обновляться через события
    Promise.all(promises).then(() => {
        // Все команды запущены
        console.log('All commands started');
    });
}

function stopProject(projectId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
        project.isRunning = false;
        project.commands.forEach(cmd => {
            cmd.isRunning = false;
        });
    }
}
</script>

<template>
    <div class="page">
        <div class="page-header">
            <h1 class="page-title">Терминал</h1>
            <button @click="addProject" class="btn-add">
                <svg viewBox="0 0 24 24" class="btn-icon">
                    <line x1="12" y1="5" x2="12" y2="19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    <line x1="5" y1="12" x2="19" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                </svg>
                Добавить проект
            </button>
        </div>

        <div v-if="projects.length === 0" class="empty-state">
            <p>Нет проектов. Нажмите "Добавить проект" для создания.</p>
        </div>

        <div v-else class="projects-container">
            <div v-for="project in projects" :key="project.id" class="project-card">
                <div class="project-header">
                    <div class="project-info">
                        <input
                            :value="project.name"
                            @input="updateProjectName(project.id, ($event.target as HTMLInputElement).value)"
                            class="project-name-input"
                            placeholder="Название проекта"
                        />
                        <input
                            :value="project.path"
                            @input="updateProjectPath(project.id, ($event.target as HTMLInputElement).value)"
                            class="project-path-input"
                            placeholder="Путь к проекту (например: /var/www/my-project)"
                        />
                    </div>
                    <div class="project-actions">
                        <button
                            @click="addCommand(project.id)"
                            :disabled="project.commands.length >= 6"
                            class="btn-add-command"
                            :title="project.commands.length >= 6 ? 'Максимум 6 команд' : 'Добавить команду'"
                        >
                            <svg viewBox="0 0 24 24" class="icon">
                                <line x1="12" y1="5" x2="12" y2="19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                <line x1="5" y1="12" x2="19" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            </svg>
                        </button>
                        <button
                            v-if="!project.isRunning"
                            @click="runProject(project.id)"
                            :disabled="project.commands.length === 0"
                            class="btn-run"
                        >
                            Запустить проект
                        </button>
                        <button
                            v-else
                            @click="stopProject(project.id)"
                            class="btn-stop"
                        >
                            Закрыть терминалы
                        </button>
                        <button @click="deleteProject(project.id)" class="btn-delete" title="Удалить проект">
                            <svg viewBox="0 0 24 24" class="icon">
                                <line x1="18" y1="6" x2="6" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                <line x1="6" y1="6" x2="18" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            </svg>
                        </button>
                    </div>
                </div>

                <div v-if="project.commands.length === 0" class="no-commands">
                    Нет команд. Нажмите "+" чтобы добавить команду.
                </div>

                <div v-if="!project.isRunning && project.commands.length > 0" class="commands-setup">
                    <div v-for="cmd in project.commands" :key="cmd.id" class="command-row">
                        <button @click="deleteCommand(project.id, cmd.id)" class="btn-delete-cmd" title="Удалить">
                            <svg viewBox="0 0 24 24" class="icon">
                                <line x1="18" y1="6" x2="6" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                <line x1="6" y1="6" x2="18" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            </svg>
                        </button>
                        <input
                            :value="cmd.command"
                            @input="updateCommand(project.id, cmd.id, ($event.target as HTMLInputElement).value)"
                            class="command-input"
                            placeholder="Введите команду"
                        />
                    </div>
                </div>

                <div v-if="project.isRunning" class="terminals-grid" :class="`grid-${project.commands.length}`">
                    <div v-for="cmd in project.commands" :key="cmd.id" class="terminal-window">
                        <div class="terminal-header">
                            <span class="terminal-title">{{ cmd.command || 'Команда' }}</span>
                            <span v-if="cmd.isRunning" class="terminal-status running">Выполняется...</span>
                            <span v-else class="terminal-status completed">Завершено</span>
                        </div>
                        <textarea
                            :value="cmd.output || 'Ожидание вывода...'"
                            :data-command-id="`${project.id}-${cmd.id}`"
                            readonly
                            class="terminal-output"
                        ></textarea>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped lang="stylus">
.page
    display flex
    flex-direction column
    padding 20px
    max-width 100%
    margin 0 auto
    height 100vh
    overflow-y auto

.page-header
    display flex
    align-items center
    justify-content space-between
    margin-bottom 20px
    padding 0 10px

.page-title
    font-size 28px
    font-weight 700
    color #0052cc
    margin 0

.btn-add
    display flex
    align-items center
    gap 6px
    padding 8px 16px
    background-color #0052cc
    color white
    border none
    border-radius 6px
    font-size 14px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover
        background-color #0747a6

.btn-icon
    width 16px
    height 16px

.empty-state
    text-align center
    padding 40px
    color #6B778C
    font-size 14px

.projects-container
    display flex
    flex-direction column
    gap 20px

.project-card
    background white
    border-radius 8px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)
    padding 20px
    transition all 0.2s ease

    &:hover
        box-shadow 0 4px 12px rgba(0, 0, 0, 0.15)

.project-header
    display flex
    align-items flex-start
    justify-content space-between
    gap 12px
    margin-bottom 16px
    padding-bottom 12px
    border-bottom 2px solid #f4f5f7

.project-info
    flex 1
    display flex
    flex-direction column
    gap 8px

.project-name-input
    width 100%
    padding 8px 12px
    border 2px solid #DFE1E6
    border-radius 6px
    font-size 16px
    font-weight 600
    color #172B4D
    transition border-color 0.2s ease

    &:focus
        outline none
        border-color #0052cc

.project-path-input
    width 100%
    padding 6px 12px
    border 1px solid #DFE1E6
    border-radius 4px
    font-size 13px
    font-family 'JetBrains Mono', 'Courier New', monospace
    color #505F79
    transition border-color 0.2s ease

    &:focus
        outline none
        border-color #0052cc

    &::placeholder
        color #97A0AF

.project-actions
    display flex
    align-items center
    gap 8px

.btn-add-command
    width 36px
    height 36px
    display flex
    align-items center
    justify-content center
    background-color #00875A
    color white
    border none
    border-radius 6px
    cursor pointer
    transition all 0.2s ease

    &:hover:not(:disabled)
        background-color #006644

    &:disabled
        background-color #DFE1E6
        cursor not-allowed

.btn-run
    padding 8px 16px
    background-color #00875A
    color white
    border none
    border-radius 6px
    font-size 14px
    font-weight 600
    cursor pointer
    transition all 0.2s ease
    white-space nowrap

    &:hover:not(:disabled)
        background-color #006644

    &:disabled
        background-color #DFE1E6
        cursor not-allowed

.btn-stop
    padding 8px 16px
    background-color #DE350B
    color white
    border none
    border-radius 6px
    font-size 14px
    font-weight 600
    cursor pointer
    transition all 0.2s ease
    white-space nowrap

    &:hover
        background-color #BF2600

.btn-delete
    width 36px
    height 36px
    display flex
    align-items center
    justify-content center
    background none
    border 1px solid #DE350B
    border-radius 6px
    cursor pointer
    color #DE350B
    transition all 0.2s ease

    &:hover
        background-color rgba(222, 53, 11, 0.1)

.icon
    width 18px
    height 18px

.no-commands
    text-align center
    padding 30px
    color #6B778C
    font-size 14px
    background #f4f5f7
    border-radius 6px

.commands-setup
    display flex
    flex-direction column
    gap 8px

.command-row
    display flex
    align-items center
    gap 8px

.btn-delete-cmd
    flex-shrink 0
    width 32px
    height 32px
    display flex
    align-items center
    justify-content center
    background none
    border none
    cursor pointer
    color #DE350B
    border-radius 4px
    transition all 0.2s ease

    &:hover
        background-color rgba(222, 53, 11, 0.1)

.command-input
    flex 1
    padding 8px 12px
    border 1px solid #DFE1E6
    border-radius 6px
    font-size 14px
    font-family 'JetBrains Mono', 'Courier New', monospace
    color #172B4D
    transition border-color 0.2s ease

    &:focus
        outline none
        border-color #0052cc

.terminals-grid
    display grid
    gap 12px

    &.grid-1
        grid-template-columns 1fr

    &.grid-2
        grid-template-columns repeat(2, 1fr)

    &.grid-3
        grid-template-columns repeat(3, 1fr)

    &.grid-4
        grid-template-columns repeat(2, 1fr)

    &.grid-5
        grid-template-columns repeat(3, 1fr)

    &.grid-6
        grid-template-columns repeat(3, 1fr)

.terminal-window
    background #1e1e1e
    border-radius 6px
    overflow hidden
    display flex
    flex-direction column
    min-height 200px
    max-height 400px

.terminal-header
    background #2d2d30
    padding 8px 12px
    display flex
    align-items center
    justify-content space-between
    border-bottom 1px solid #3e3e42

.terminal-title
    font-size 13px
    color #cccccc
    font-family 'JetBrains Mono', 'Courier New', monospace
    white-space nowrap
    overflow hidden
    text-overflow ellipsis
    flex 1

.terminal-status
    font-size 11px
    padding 2px 8px
    border-radius 4px

    &.running
        background-color #00875A
        color white

    &.completed
        background-color #6B778C
        color white

.terminal-output
    flex 1
    padding 12px
    background #1e1e1e
    color #d4d4d4
    font-family 'JetBrains Mono', 'Courier New', monospace
    font-size 13px
    border none
    resize none
    overflow-y auto
    line-height 1.5
    white-space pre-wrap
    word-break break-all

    &:focus
        outline none
</style>
