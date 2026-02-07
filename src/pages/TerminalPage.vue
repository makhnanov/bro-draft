<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { WebviewWindow, getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import draggable from 'vuedraggable';

interface Command {
    id: number;
    command: string;
    workingDirectory: string;
    isRunning: boolean;
    popupLabel: string | null; // Label of the popup window when running
}

interface SavedLayout {
    id: string;
    type: 'terminal' | 'container';
    commandId?: number;
    direction?: 'horizontal' | 'vertical';
    children?: SavedLayout[];
}

interface WindowState {
    width: number;
    height: number;
    x: number;
    y: number;
}

interface Project {
    id: number;
    name: string;
    commands: Command[];
    isRunning: boolean;
    layout?: SavedLayout | null;
    windowState?: WindowState | null;
}

const projects = ref<Project[]>([]);
let projectsUpdateUnlisten: UnlistenFn | null = null;

// Dragging state for setup mode
const isDragging = ref(false);

// Load/Save projects
function loadProjects() {
    const saved = localStorage.getItem('terminal_projects_v4');
    if (saved) {
        try {
            const parsed = JSON.parse(saved);
            projects.value = parsed.map((p: any) => ({
                ...p,
                isRunning: false,
                layout: p.layout || null,
                windowState: p.windowState || null,
                commands: p.commands.map((c: any) => ({
                    ...c,
                    isRunning: false,
                    popupLabel: null,
                })),
            }));
        } catch (e) {
            console.error('Failed to load projects:', e);
        }
    }
}

function saveProjects() {
    const toSave = projects.value.map(p => ({
        id: p.id,
        name: p.name,
        commands: p.commands.map(c => ({
            id: c.id,
            command: c.command,
            workingDirectory: c.workingDirectory,
        })),
        layout: p.layout || null,
        windowState: p.windowState || null,
    }));
    localStorage.setItem('terminal_projects_v4', JSON.stringify(toSave));
}

// Sync with popup window changes
function onStorageChange(event: StorageEvent) {
    if (event.key === 'terminal_projects_v4' && event.newValue) {
        try {
            const parsed = JSON.parse(event.newValue);
            // Update commands and layout for each project (preserve isRunning state)
            for (const savedProject of parsed) {
                const project = projects.value.find(p => p.id === savedProject.id);
                if (project) {
                    // Update commands list
                    const newCommands = savedProject.commands.map((c: any) => {
                        const existing = project.commands.find(ec => ec.id === c.id);
                        return {
                            id: c.id,
                            command: c.command,
                            workingDirectory: c.workingDirectory,
                            isRunning: existing?.isRunning || false,
                            popupLabel: existing?.popupLabel || null,
                        };
                    });
                    project.commands = newCommands;
                    project.layout = savedProject.layout || null;
                    project.windowState = savedProject.windowState || null;
                }
            }
        } catch (e) {
            console.error('Failed to sync projects:', e);
        }
    }
}

// Project management
function addProject() {
    const newProject: Project = {
        id: Date.now(),
        name: 'New Project',
        commands: [],
        isRunning: false,
    };
    projects.value.push(newProject);
    saveProjects();
}

function deleteProject(projectId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (project && project.isRunning) {
        stopProject(projectId);
    }
    projects.value = projects.value.filter(p => p.id !== projectId);
    saveProjects();
}

function updateProjectName(projectId: number, name: string) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
        project.name = name;
        saveProjects();
    }
}

// Command management
function addCommand(projectId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (project && project.commands.length < 10) {
        const newCommand: Command = {
            id: Date.now(),
            command: '',
            workingDirectory: '',
            isRunning: false,
            popupLabel: null,
        };
        project.commands.push(newCommand);
        // Reset layout when commands change from main window
        project.layout = null;
        saveProjects();
    }
}

function deleteCommand(projectId: number, commandId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
        project.commands = project.commands.filter(c => c.id !== commandId);
        // Reset layout when commands change from main window
        project.layout = null;
        saveProjects();
    }
}

function updateCommand(projectId: number, commandId: number, field: 'command' | 'workingDirectory', value: string) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
        const cmd = project.commands.find(c => c.id === commandId);
        if (cmd) {
            cmd[field] = value;
            saveProjects();
        }
    }
}

// Run/Stop project
async function runProject(projectId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (!project || project.commands.length === 0) return;

    project.isRunning = true;

    // Open a single popup window with all terminals
    await openProjectPopup(project);
}

// Open a popup window with all project terminals
async function openProjectPopup(project: Project) {
    try {
        const popupLabel = `terminal-project-${project.id}-${Date.now()}`;

        const params = new URLSearchParams();
        params.set('projectId', project.id.toString());
        params.set('name', project.name);

        const url = `index.html#/terminal-project-popup?${params.toString()}`;

        // Use saved window state or defaults
        const ws = project.windowState;
        console.log('[MAIN] Opening popup, saved windowState:', ws);
        const windowOptions: any = {
            url: url,
            title: project.name,
            width: ws?.width || 1200,
            height: ws?.height || 800,
            resizable: true,
            decorations: false,
            backgroundColor: '#2d2d30',
        };

        // If we have saved position, use it; otherwise center
        if (ws?.x !== undefined && ws?.y !== undefined) {
            windowOptions.x = ws.x;
            windowOptions.y = ws.y;
        } else {
            windowOptions.center = true;
        }
        console.log('[MAIN] Window options:', windowOptions);

        const webview = new WebviewWindow(popupLabel, windowOptions);

        // Store popup label on project level
        for (const cmd of project.commands) {
            cmd.popupLabel = popupLabel;
            cmd.isRunning = true;
        }

        webview.once('tauri://created', () => {
            console.log('Terminal project popup window created:', project.name);
        });

        webview.once('tauri://error', (e) => {
            console.error('Failed to create terminal project popup window:', e);
            for (const cmd of project.commands) {
                cmd.popupLabel = null;
                cmd.isRunning = false;
            }
            project.isRunning = false;
        });

        // Handle window close (Alt+F4, close button, etc.)
        webview.once('tauri://destroyed', () => {
            console.log('Terminal project popup window closed:', project.name);
            for (const cmd of project.commands) {
                cmd.popupLabel = null;
                cmd.isRunning = false;
            }
            project.isRunning = false;
        });
    } catch (error) {
        console.error('Failed to open terminal project popup:', error);
    }
}

async function stopProject(projectId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (!project) return;

    // Close all popup windows for this project
    const allWindows = await getAllWebviewWindows();
    for (const cmd of project.commands) {
        if (cmd.popupLabel) {
            const win = allWindows.find(w => w.label === cmd.popupLabel);
            if (win) {
                try {
                    await win.close();
                } catch (error) {
                    console.error('Failed to close terminal window:', error);
                }
            }
            cmd.popupLabel = null;
        }
        cmd.isRunning = false;
    }

    project.isRunning = false;
}

// Drag & Drop handlers for setup mode
function onDragStart() {
    isDragging.value = true;
}

function onDragEnd() {
    isDragging.value = false;
    saveProjects();
}

// Check which popup windows are still open and sync state
async function syncRunningState() {
    const allWindows = await getAllWebviewWindows();

    for (const project of projects.value) {
        // Check if any popup for this project is still open
        const projectPopupPrefix = `terminal-project-${project.id}-`;
        const hasOpenPopup = allWindows.some(w => w.label.startsWith(projectPopupPrefix));

        if (hasOpenPopup) {
            project.isRunning = true;
            // Find the actual popup label
            const popupWindow = allWindows.find(w => w.label.startsWith(projectPopupPrefix));
            if (popupWindow) {
                for (const cmd of project.commands) {
                    cmd.isRunning = true;
                    cmd.popupLabel = popupWindow.label;
                }
                // Re-attach destroyed listener
                popupWindow.once('tauri://destroyed', () => {
                    console.log('Terminal project popup window closed:', project.name);
                    for (const cmd of project.commands) {
                        cmd.popupLabel = null;
                        cmd.isRunning = false;
                    }
                    project.isRunning = false;
                });
            }
        } else {
            project.isRunning = false;
            for (const cmd of project.commands) {
                cmd.isRunning = false;
                cmd.popupLabel = null;
            }
        }
    }
}

onMounted(async () => {
    loadProjects();
    await syncRunningState();
    window.addEventListener('storage', onStorageChange);
    // Listen for updates from popup windows
    projectsUpdateUnlisten = await listen('terminal-projects-updated', () => {
        // Reload projects from localStorage
        const saved = localStorage.getItem('terminal_projects_v4');
        if (saved) {
            try {
                const parsed = JSON.parse(saved);
                for (const savedProject of parsed) {
                    const project = projects.value.find(p => p.id === savedProject.id);
                    if (project) {
                        const newCommands = savedProject.commands.map((c: any) => {
                            const existing = project.commands.find(ec => ec.id === c.id);
                            return {
                                id: c.id,
                                command: c.command,
                                workingDirectory: c.workingDirectory,
                                isRunning: existing?.isRunning || false,
                                popupLabel: existing?.popupLabel || null,
                            };
                        });
                        project.commands = newCommands;
                        project.layout = savedProject.layout || null;
                    }
                }
            } catch (e) {
                console.error('Failed to sync projects:', e);
            }
        }
    });
});

onUnmounted(() => {
    window.removeEventListener('storage', onStorageChange);
    if (projectsUpdateUnlisten) projectsUpdateUnlisten();
    // Don't close popup windows - they should be independent
});
</script>

<template>
    <div class="page">
        <div class="page-header">
            <h1 class="page-title">Workspaces</h1>
            <button @click="addProject" class="btn-add">
                <svg viewBox="0 0 24 24" class="btn-icon">
                    <line x1="12" y1="5" x2="12" y2="19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    <line x1="5" y1="12" x2="19" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                </svg>
                Add Project
            </button>
        </div>

        <div v-if="projects.length === 0" class="empty-state">
            <p>No projects. Click "Add Project" to create one.</p>
        </div>

        <div v-else class="projects-container">
            <div v-for="project in projects" :key="project.id" class="project-card">
                <div class="project-header">
                    <div class="project-info">
                        <input
                            :value="project.name"
                            @input="updateProjectName(project.id, ($event.target as HTMLInputElement).value)"
                            class="project-name-input"
                            placeholder="Project name"
                        />
                    </div>
                    <div class="project-actions">
                        <button
                            @click="addCommand(project.id)"
                            :disabled="project.commands.length >= 10 || project.isRunning"
                            class="btn-add-cmd"
                            title="Add command"
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
                            Run
                        </button>
                        <button
                            v-else
                            @click="stopProject(project.id)"
                            class="btn-stop"
                        >
                            Stop
                        </button>
                        <button
                            @click="deleteProject(project.id)"
                            class="btn-delete"
                            title="Delete project"
                            :disabled="project.isRunning"
                        >
                            <svg viewBox="0 0 24 24" class="icon">
                                <line x1="18" y1="6" x2="6" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                <line x1="6" y1="6" x2="18" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            </svg>
                        </button>
                    </div>
                </div>

                <!-- Commands Setup (when not running) -->
                <div v-if="!project.isRunning" class="commands-setup">
                    <div v-if="project.commands.length === 0" class="no-commands">
                        No commands. Click "+" to add a command.
                    </div>
                    <draggable
                        v-else
                        v-model="project.commands"
                        item-key="id"
                        handle=".drag-handle"
                        ghost-class="ghost"
                        @start="onDragStart"
                        @end="onDragEnd"
                        class="commands-list"
                    >
                        <template #item="{ element: cmd }">
                            <div class="command-row">
                                <div class="drag-handle" title="Drag to reorder">
                                    <svg viewBox="0 0 24 24" class="icon">
                                        <circle cx="9" cy="6" r="1.5" fill="currentColor"/>
                                        <circle cx="15" cy="6" r="1.5" fill="currentColor"/>
                                        <circle cx="9" cy="12" r="1.5" fill="currentColor"/>
                                        <circle cx="15" cy="12" r="1.5" fill="currentColor"/>
                                        <circle cx="9" cy="18" r="1.5" fill="currentColor"/>
                                        <circle cx="15" cy="18" r="1.5" fill="currentColor"/>
                                    </svg>
                                </div>
                                <div class="command-inputs">
                                    <div class="input-group command-group">
                                        <label class="input-label">Command</label>
                                        <input
                                            :value="cmd.command"
                                            @input="updateCommand(project.id, cmd.id, 'command', ($event.target as HTMLInputElement).value)"
                                            class="command-input"
                                            placeholder="e.g. npm run dev"
                                        />
                                    </div>
                                    <div class="input-group workdir-group">
                                        <label class="input-label">Working Directory</label>
                                        <input
                                            :value="cmd.workingDirectory"
                                            @input="updateCommand(project.id, cmd.id, 'workingDirectory', ($event.target as HTMLInputElement).value)"
                                            class="workdir-input"
                                            placeholder="e.g. /home/user/project"
                                        />
                                    </div>
                                </div>
                                <button @click="deleteCommand(project.id, cmd.id)" class="btn-delete-cmd" title="Delete">
                                    <svg viewBox="0 0 24 24" class="icon">
                                        <line x1="18" y1="6" x2="6" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                        <line x1="6" y1="6" x2="18" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                    </svg>
                                </button>
                            </div>
                        </template>
                    </draggable>
                </div>

                <!-- Running state info -->
                <div v-if="project.isRunning" class="running-info">
                    <div class="running-message">
                        <svg viewBox="0 0 24 24" class="running-icon">
                            <rect x="3" y="3" width="18" height="18" rx="2" stroke="currentColor" stroke-width="2" fill="none"/>
                            <path d="M7 15l3-3 2 2 5-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
                        </svg>
                        <span>{{ project.commands.length }} terminal{{ project.commands.length > 1 ? 's' : '' }} running in separate window{{ project.commands.length > 1 ? 's' : '' }}</span>
                    </div>
                    <div class="running-commands">
                        <div v-for="cmd in project.commands" :key="cmd.id" class="running-command-item">
                            <span class="running-dot"></span>
                            <span class="running-command-text">{{ cmd.command || '(empty command)' }}</span>
                        </div>
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
    height 100vh
    overflow-y auto
    background #f5f5f5

.page-header
    display flex
    align-items center
    justify-content space-between
    margin-bottom 20px

.page-title
    font-size 24px
    font-weight 700
    color #333
    margin 0

.btn-add
    display flex
    align-items center
    gap 6px
    padding 8px 16px
    background-color #0078d4
    color white
    border none
    border-radius 6px
    font-size 14px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover
        background-color #106ebe

.btn-icon
    width 16px
    height 16px

.empty-state
    text-align center
    padding 60px
    color #666
    font-size 14px
    background white
    border-radius 8px

.projects-container
    display flex
    flex-direction column
    gap 20px

.project-card
    background white
    border-radius 8px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)
    overflow hidden

.project-header
    display flex
    align-items center
    justify-content space-between
    padding 16px
    background #f8f9fa
    border-bottom 1px solid #e9ecef

.project-info
    flex 1
    margin-right 16px

.project-name-input
    width 100%
    max-width 300px
    padding 8px 12px
    border 1px solid #ddd
    border-radius 6px
    font-size 16px
    font-weight 600
    color #333

    &:focus
        outline none
        border-color #0078d4

    &:disabled
        background #f5f5f5
        cursor not-allowed

.project-actions
    display flex
    align-items center
    gap 8px

.btn-add-cmd
    width 36px
    height 36px
    display flex
    align-items center
    justify-content center
    background-color #28a745
    color white
    border none
    border-radius 6px
    cursor pointer
    transition all 0.2s ease

    &:hover:not(:disabled)
        background-color #218838

    &:disabled
        background-color #ccc
        cursor not-allowed

.btn-run
    padding 8px 20px
    background-color #28a745
    color white
    border none
    border-radius 6px
    font-size 14px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover:not(:disabled)
        background-color #218838

    &:disabled
        background-color #ccc
        cursor not-allowed

.btn-stop
    padding 8px 20px
    background-color #dc3545
    color white
    border none
    border-radius 6px
    font-size 14px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover
        background-color #c82333

.btn-delete
    width 36px
    height 36px
    display flex
    align-items center
    justify-content center
    background none
    border 1px solid #dc3545
    border-radius 6px
    cursor pointer
    color #dc3545
    transition all 0.2s ease

    &:hover:not(:disabled)
        background-color rgba(220, 53, 69, 0.1)

    &:disabled
        opacity 0.5
        cursor not-allowed

.icon
    width 18px
    height 18px

.commands-setup
    padding 16px

.no-commands
    text-align center
    padding 30px
    color #666
    font-size 14px
    background #f8f9fa
    border-radius 6px

.commands-list
    display flex
    flex-direction column
    gap 12px

.command-row
    display flex
    align-items flex-start
    gap 8px
    padding 12px
    background #f8f9fa
    border-radius 8px
    border 1px solid #e9ecef

    &.ghost
        opacity 0.5
        background #e3f2fd

.drag-handle
    flex-shrink 0
    width 24px
    height 24px
    display flex
    align-items center
    justify-content center
    cursor grab
    color #999
    margin-top 8px

    &:active
        cursor grabbing

    .icon
        width 16px
        height 16px

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
    color #dc3545
    border-radius 4px
    margin-top 4px
    transition all 0.2s ease

    &:hover
        background-color rgba(220, 53, 69, 0.1)

.command-inputs
    flex 1
    display flex
    flex-direction row
    gap 12px

.input-group
    display flex
    flex-direction column
    gap 4px

.command-group
    flex 1

.workdir-group
    flex 0 0 20%
    min-width 150px

.input-label
    font-size 12px
    font-weight 600
    color #666
    padding-left 2px

.command-input
.workdir-input
    width 100%
    padding 10px 12px
    border 1px solid #ddd
    border-radius 6px
    font-size 14px
    font-family 'JetBrains Mono', 'Consolas', monospace
    color #333
    transition border-color 0.2s ease

    &:focus
        outline none
        border-color #0078d4

    &::placeholder
        color #999

.workdir-input
    font-size 12px
    padding 8px 12px
    color #666

.running-info
    padding 20px
    background #e8f5e9
    border-radius 8px
    margin 16px

.running-message
    display flex
    align-items center
    gap 12px
    color #2e7d32
    font-size 16px
    font-weight 600
    margin-bottom 16px

.running-icon
    width 24px
    height 24px
    flex-shrink 0

.running-commands
    display flex
    flex-direction column
    gap 8px

.running-command-item
    display flex
    align-items center
    gap 10px
    padding 8px 12px
    background rgba(255, 255, 255, 0.7)
    border-radius 6px

.running-dot
    width 8px
    height 8px
    background #4caf50
    border-radius 50%
    animation pulse 2s infinite

.running-command-text
    font-family 'JetBrains Mono', 'Consolas', monospace
    font-size 13px
    color #333

@keyframes pulse
    0%, 100%
        opacity 1
    50%
        opacity 0.5

</style>
