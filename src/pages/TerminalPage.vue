<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import draggable from 'vuedraggable';
import '@xterm/xterm/css/xterm.css';

interface PtyOutputEvent {
    session_id: string;
    data: string;
}

interface Command {
    id: number;
    command: string;
    workingDirectory: string;
    sessionId: string | null;
    terminal: Terminal | null;
    fitAddon: FitAddon | null;
    isRunning: boolean;
}

interface TerminalRow {
    id: string;
    commands: Command[];
}

interface Project {
    id: number;
    name: string;
    commands: Command[]; // For setup mode (flat list)
    rows: TerminalRow[]; // For running mode (grouped into rows)
    isRunning: boolean;
}

const projects = ref<Project[]>([]);
let outputUnlisten: UnlistenFn | null = null;
let resizeObservers: Map<string, ResizeObserver> = new Map();

// Dragging state
const isDragging = ref(false);
const draggedTerminal = ref<{ projectId: number; rowId: string; cmdId: number } | null>(null);
const dropTarget = ref<{ projectId: number; rowId: string; cmdId: number; position: 'left' | 'right' | 'top' | 'bottom' } | null>(null);

// Load/Save projects
function loadProjects() {
    const saved = localStorage.getItem('terminal_projects_v4');
    if (saved) {
        try {
            const parsed = JSON.parse(saved);
            projects.value = parsed.map((p: any) => ({
                ...p,
                isRunning: false,
                rows: [],
                commands: p.commands.map((c: any) => ({
                    ...c,
                    sessionId: null,
                    terminal: null,
                    fitAddon: null,
                    isRunning: false,
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
    }));
    localStorage.setItem('terminal_projects_v4', JSON.stringify(toSave));
}

// Project management
function addProject() {
    const newProject: Project = {
        id: Date.now(),
        name: 'New Project',
        commands: [],
        rows: [],
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
            sessionId: null,
            terminal: null,
            fitAddon: null,
            isRunning: false,
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

// Terminal initialization
async function initTerminal(project: Project, cmd: Command) {
    const containerId = `terminal-${project.id}-${cmd.id}`;
    const container = document.getElementById(containerId);
    if (!container) return;

    const terminal = new Terminal({
        cursorBlink: true,
        fontSize: 13,
        fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
        theme: {
            background: '#1e1e1e',
            foreground: '#d4d4d4',
            cursor: '#ffffff',
            selectionBackground: '#264f78',
        },
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(container);

    setTimeout(() => fitAddon.fit(), 100);

    cmd.terminal = terminal;
    cmd.fitAddon = fitAddon;

    // Handle input
    terminal.onData(async (data) => {
        if (cmd.sessionId) {
            try {
                await invoke('write_to_pty', {
                    sessionId: cmd.sessionId,
                    data,
                });
            } catch (error) {
                console.error('Failed to write to PTY:', error);
            }
        }
    });

    // Setup resize observer
    const observer = new ResizeObserver(() => {
        if (cmd.fitAddon && cmd.sessionId) {
            cmd.fitAddon.fit();
            invoke('resize_pty', {
                sessionId: cmd.sessionId,
                rows: cmd.terminal?.rows || 24,
                cols: cmd.terminal?.cols || 80,
            }).catch(console.error);
        }
    });
    observer.observe(container);
    resizeObservers.set(containerId, observer);

    // Create PTY session
    try {
        const sessionId = await invoke<string>('create_pty_session', {
            rows: terminal.rows,
            cols: terminal.cols,
            workingDirectory: cmd.workingDirectory || null,
        });
        cmd.sessionId = sessionId;

        // Execute the command
        if (cmd.command) {
            await invoke('write_to_pty', {
                sessionId,
                data: cmd.command + '\n',
            });
        }
    } catch (error) {
        console.error('Failed to create PTY session:', error);
        terminal.writeln(`Error: ${error}`);
    }
}

// Run/Stop project
async function runProject(projectId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (!project || project.commands.length === 0) return;

    project.isRunning = true;

    // Convert commands to rows (initially one command per row)
    project.rows = project.commands.map(cmd => ({
        id: `row-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        commands: [{
            ...cmd,
            isRunning: true,
        }],
    }));

    await nextTick();

    // Initialize terminals for all commands in all rows
    for (const row of project.rows) {
        for (const cmd of row.commands) {
            await initTerminal(project, cmd);
        }
    }
}

async function stopProject(projectId: number) {
    const project = projects.value.find(p => p.id === projectId);
    if (!project) return;

    // Kill all PTY sessions in all rows
    for (const row of project.rows) {
        for (const cmd of row.commands) {
            if (cmd.sessionId) {
                try {
                    await invoke('kill_pty_session', { sessionId: cmd.sessionId });
                } catch (error) {
                    console.error('Failed to kill PTY:', error);
                }
            }

            const containerId = `terminal-${project.id}-${cmd.id}`;
            const observer = resizeObservers.get(containerId);
            if (observer) {
                observer.disconnect();
                resizeObservers.delete(containerId);
            }

            if (cmd.terminal) {
                cmd.terminal.dispose();
                cmd.terminal = null;
            }
            cmd.fitAddon = null;
            cmd.sessionId = null;
            cmd.isRunning = false;
        }
    }

    // Clear rows
    project.rows = [];
    project.isRunning = false;
}

// Open new terminal popup window
async function openNewTerminalPopup(project: Project) {
    try {
        const popupId = `terminal-popup-${Date.now()}`;
        const title = `${project.name} - Terminal`;

        new WebviewWindow(popupId, {
            url: `index.html#/terminal-popup?title=${encodeURIComponent(title)}`,
            title: title,
            width: 900,
            height: 600,
            resizable: true,
            center: true,
        });
    } catch (error) {
        console.error('Failed to open popup:', error);
    }
}

// Add new terminal to the right of an existing one in the same row
async function addTerminalToRight(project: Project, row: TerminalRow, afterCmd: Command) {
    // Create new command
    const newCmd: Command = {
        id: Date.now(),
        command: '',
        workingDirectory: afterCmd.workingDirectory, // Inherit working directory
        sessionId: null,
        terminal: null,
        fitAddon: null,
        isRunning: true,
    };

    // Find position and insert after the current command
    const cmdIndex = row.commands.findIndex(c => c.id === afterCmd.id);
    row.commands.splice(cmdIndex + 1, 0, newCmd);

    // Also add to project commands list for persistence
    project.commands.push({
        ...newCmd,
        isRunning: false,
    });
    saveProjects();

    await nextTick();

    // Initialize the new terminal
    await initTerminal(project, newCmd);

    // Re-fit all terminals in the row
    for (const cmd of row.commands) {
        if (cmd.fitAddon) {
            cmd.fitAddon.fit();
        }
    }
}

// Drag & Drop handlers for setup mode
function onDragStart() {
    isDragging.value = true;
}

function onDragEnd() {
    isDragging.value = false;
    saveProjects();
}

// Drag & Drop handlers for running terminals with edge detection
function startTerminalDrag(event: MouseEvent, project: Project, row: TerminalRow, cmd: Command) {
    event.preventDefault();
    isDragging.value = true;
    draggedTerminal.value = { projectId: project.id, rowId: row.id, cmdId: cmd.id };
    document.body.classList.add('terminal-dragging');

    document.addEventListener('mousemove', onTerminalDrag);
    document.addEventListener('mouseup', endTerminalDrag);
}

function onTerminalDrag(event: MouseEvent) {
    if (!draggedTerminal.value) return;

    const project = projects.value.find(p => p.id === draggedTerminal.value!.projectId);
    if (!project) return;

    // Find element under cursor
    const elementsUnder = document.elementsFromPoint(event.clientX, event.clientY);
    const terminalEl = elementsUnder.find(el => el.classList.contains('terminal-window')) as HTMLElement | undefined;

    if (terminalEl) {
        const cmdId = parseInt(terminalEl.dataset.cmdId || '0');
        const rowId = terminalEl.dataset.rowId || '';

        // Don't show drop target on the dragged terminal itself
        if (cmdId === draggedTerminal.value.cmdId) {
            dropTarget.value = null;
            return;
        }

        const rect = terminalEl.getBoundingClientRect();
        const relX = (event.clientX - rect.left) / rect.width;
        const relY = (event.clientY - rect.top) / rect.height;

        // Determine position based on cursor location
        let position: 'left' | 'right' | 'top' | 'bottom';
        if (relY < 0.25) {
            position = 'top';
        } else if (relY > 0.75) {
            position = 'bottom';
        } else if (relX < 0.5) {
            position = 'left';
        } else {
            position = 'right';
        }

        dropTarget.value = { projectId: project.id, rowId, cmdId, position };
    } else {
        dropTarget.value = null;
    }
}

function endTerminalDrag() {
    if (draggedTerminal.value && dropTarget.value) {
        performDrop();
    }

    isDragging.value = false;
    draggedTerminal.value = null;
    dropTarget.value = null;
    document.body.classList.remove('terminal-dragging');

    document.removeEventListener('mousemove', onTerminalDrag);
    document.removeEventListener('mouseup', endTerminalDrag);
}

function performDrop() {
    if (!draggedTerminal.value || !dropTarget.value) return;

    const project = projects.value.find(p => p.id === draggedTerminal.value!.projectId);
    if (!project) return;

    // Find source row and command
    const sourceRowIndex = project.rows.findIndex(r => r.id === draggedTerminal.value!.rowId);
    if (sourceRowIndex === -1) return;

    const sourceRow = project.rows[sourceRowIndex];
    const sourceCmdIndex = sourceRow.commands.findIndex(c => c.id === draggedTerminal.value!.cmdId);
    if (sourceCmdIndex === -1) return;

    const [movedCmd] = sourceRow.commands.splice(sourceCmdIndex, 1);

    // Find target row
    const targetRowIndex = project.rows.findIndex(r => r.id === dropTarget.value!.rowId);
    if (targetRowIndex === -1) return;

    const targetRow = project.rows[targetRowIndex];
    const targetCmdIndex = targetRow.commands.findIndex(c => c.id === dropTarget.value!.cmdId);

    const position = dropTarget.value.position;

    if (position === 'left' || position === 'right') {
        // Add to same row
        const insertIndex = position === 'left' ? targetCmdIndex : targetCmdIndex + 1;
        targetRow.commands.splice(insertIndex, 0, movedCmd);
    } else {
        // Create new row
        const newRow: TerminalRow = {
            id: `row-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
            commands: [movedCmd],
        };

        const insertRowIndex = position === 'top' ? targetRowIndex : targetRowIndex + 1;
        project.rows.splice(insertRowIndex, 0, newRow);
    }

    // Remove empty source row
    if (sourceRow.commands.length === 0) {
        const idx = project.rows.findIndex(r => r.id === sourceRow.id);
        if (idx !== -1) {
            project.rows.splice(idx, 1);
        }
    }

    // Recreate terminal on new DOM element
    nextTick(async () => {
        await recreateTerminal(project, movedCmd);

        // Re-fit all other terminals
        for (const row of project.rows) {
            for (const cmd of row.commands) {
                if (cmd.fitAddon && cmd.terminal && cmd.id !== movedCmd.id) {
                    cmd.fitAddon.fit();
                }
            }
        }
    });
}

// Recreate terminal instance while keeping PTY session
async function recreateTerminal(project: Project, cmd: Command) {
    const containerId = `terminal-${project.id}-${cmd.id}`;
    const container = document.getElementById(containerId);
    if (!container) return;

    // Disconnect old ResizeObserver
    const oldObserver = resizeObservers.get(containerId);
    if (oldObserver) {
        oldObserver.disconnect();
        resizeObservers.delete(containerId);
    }

    // Save terminal buffer content before disposing
    let savedContent: string[] = [];
    if (cmd.terminal) {
        const buffer = cmd.terminal.buffer.active;
        for (let i = 0; i < buffer.length; i++) {
            const line = buffer.getLine(i);
            if (line) {
                savedContent.push(line.translateToString(true));
            }
        }
        // Remove trailing empty lines
        while (savedContent.length > 0 && savedContent[savedContent.length - 1].trim() === '') {
            savedContent.pop();
        }
    }

    // Dispose old terminal
    if (cmd.terminal) {
        cmd.terminal.dispose();
    }

    // Create new terminal instance
    const terminal = new Terminal({
        cursorBlink: true,
        fontSize: 13,
        fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
        theme: {
            background: '#1e1e1e',
            foreground: '#d4d4d4',
            cursor: '#ffffff',
            selectionBackground: '#264f78',
        },
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(container);

    // Restore saved content
    if (savedContent.length > 0) {
        terminal.write(savedContent.join('\r\n') + '\r\n');
    }

    setTimeout(() => fitAddon.fit(), 50);

    cmd.terminal = terminal;
    cmd.fitAddon = fitAddon;

    // Handle input - connect to existing PTY session
    terminal.onData(async (data) => {
        if (cmd.sessionId) {
            try {
                await invoke('write_to_pty', {
                    sessionId: cmd.sessionId,
                    data,
                });
            } catch (error) {
                console.error('Failed to write to PTY:', error);
            }
        }
    });

    // Setup new ResizeObserver
    const newObserver = new ResizeObserver(() => {
        if (cmd.fitAddon && cmd.sessionId) {
            cmd.fitAddon.fit();
            invoke('resize_pty', {
                sessionId: cmd.sessionId,
                rows: cmd.terminal?.rows || 24,
                cols: cmd.terminal?.cols || 80,
            }).catch(console.error);
        }
    });
    newObserver.observe(container);
    resizeObservers.set(containerId, newObserver);

    // Resize PTY to match new terminal size
    if (cmd.sessionId) {
        await invoke('resize_pty', {
            sessionId: cmd.sessionId,
            rows: terminal.rows,
            cols: terminal.cols,
        }).catch(console.error);
    }

    terminal.focus();
}

// Check if this terminal is being dragged
function isDraggedTerminal(cmdId: number): boolean {
    return draggedTerminal.value?.cmdId === cmdId;
}

// Check if this terminal has a drop target
function getDropPosition(rowId: string, cmdId: number): string | null {
    if (dropTarget.value?.rowId === rowId && dropTarget.value?.cmdId === cmdId) {
        return dropTarget.value.position;
    }
    return null;
}

onMounted(async () => {
    loadProjects();

    outputUnlisten = await listen<PtyOutputEvent>('pty-output', (event) => {
        for (const project of projects.value) {
            // Search in rows for running terminals
            for (const row of project.rows) {
                const cmd = row.commands.find(c => c.sessionId === event.payload.session_id);
                if (cmd && cmd.terminal) {
                    cmd.terminal.write(event.payload.data);
                    return;
                }
            }
        }
    });
});

onUnmounted(() => {
    if (outputUnlisten) outputUnlisten();

    projects.value.forEach(project => {
        project.commands.forEach(cmd => {
            if (cmd.sessionId) {
                invoke('kill_pty_session', { sessionId: cmd.sessionId }).catch(console.error);
            }
            if (cmd.terminal) {
                cmd.terminal.dispose();
            }
        });
    });

    resizeObservers.forEach(observer => observer.disconnect());
});
</script>

<template>
    <div class="page">
        <div class="page-header">
            <h1 class="page-title">Terminal</h1>
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
                            v-if="project.isRunning"
                            @click="openNewTerminalPopup(project)"
                            class="btn-popout-header"
                            title="Open terminal in new window"
                        >
                            <svg viewBox="0 0 24 24" class="icon">
                                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
                                <polyline points="15 3 21 3 21 9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
                                <line x1="10" y1="14" x2="21" y2="3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
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
                                    <input
                                        :value="cmd.command"
                                        @input="updateCommand(project.id, cmd.id, 'command', ($event.target as HTMLInputElement).value)"
                                        class="command-input"
                                        placeholder="Command (e.g. npm run dev)"
                                    />
                                    <input
                                        :value="cmd.workingDirectory"
                                        @input="updateCommand(project.id, cmd.id, 'workingDirectory', ($event.target as HTMLInputElement).value)"
                                        class="workdir-input"
                                        placeholder="Working directory (optional)"
                                    />
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

                <!-- Terminals Grid (when running) -->
                <div v-if="project.isRunning" class="terminals-grid">
                    <div
                        v-for="row in project.rows"
                        :key="row.id"
                        class="terminal-row"
                    >
                        <div
                            v-for="cmd in row.commands"
                            :key="cmd.id"
                            class="terminal-window"
                            :class="{
                                'is-dragging': isDraggedTerminal(cmd.id),
                                'drop-left': getDropPosition(row.id, cmd.id) === 'left',
                                'drop-right': getDropPosition(row.id, cmd.id) === 'right',
                                'drop-top': getDropPosition(row.id, cmd.id) === 'top',
                                'drop-bottom': getDropPosition(row.id, cmd.id) === 'bottom',
                            }"
                            :style="{ flex: `1 1 ${100 / row.commands.length}%` }"
                            :data-cmd-id="cmd.id"
                            :data-row-id="row.id"
                        >
                            <div class="terminal-header">
                                <div
                                    class="terminal-drag-handle"
                                    title="Drag to rearrange"
                                    @mousedown="(e) => startTerminalDrag(e, project, row, cmd)"
                                >
                                    <svg viewBox="0 0 24 24" class="icon">
                                        <circle cx="5" cy="9" r="1.5" fill="currentColor"/>
                                        <circle cx="12" cy="9" r="1.5" fill="currentColor"/>
                                        <circle cx="19" cy="9" r="1.5" fill="currentColor"/>
                                        <circle cx="5" cy="15" r="1.5" fill="currentColor"/>
                                        <circle cx="12" cy="15" r="1.5" fill="currentColor"/>
                                        <circle cx="19" cy="15" r="1.5" fill="currentColor"/>
                                    </svg>
                                </div>
                                <span class="terminal-title">{{ cmd.command || 'Terminal' }}</span>
                                <div class="terminal-actions">
                                    <button
                                        @click="addTerminalToRight(project, row, cmd)"
                                        class="btn-add-terminal"
                                        title="Add terminal to the right"
                                    >
                                        <svg viewBox="0 0 24 24" class="icon">
                                            <line x1="12" y1="5" x2="12" y2="19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                            <line x1="5" y1="12" x2="19" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                        </svg>
                                    </button>
                                </div>
                            </div>
                            <div :id="`terminal-${project.id}-${cmd.id}`" class="terminal-content"></div>
                            <!-- Drop zone indicators -->
                            <div class="drop-indicator drop-indicator-left"></div>
                            <div class="drop-indicator drop-indicator-right"></div>
                            <div class="drop-indicator drop-indicator-top"></div>
                            <div class="drop-indicator drop-indicator-bottom"></div>
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
    flex-direction column
    gap 8px

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

.terminals-grid
    padding 8px
    background #2d2d30
    display flex
    flex-direction column
    gap 8px
    min-height 300px

.terminal-row
    display flex
    gap 8px
    min-height 250px

.terminal-window
    display flex
    flex-direction column
    background #1e1e1e
    border-radius 6px
    overflow hidden
    min-height 250px
    position relative
    transition all 0.15s ease

    &.is-dragging
        opacity 0.5

    &.drop-left .drop-indicator-left,
    &.drop-right .drop-indicator-right,
    &.drop-top .drop-indicator-top,
    &.drop-bottom .drop-indicator-bottom
        opacity 1

.drop-indicator
    position absolute
    opacity 0
    transition opacity 0.2s ease
    pointer-events none
    z-index 10

.drop-indicator-left
    left 0
    top 0
    bottom 0
    width 40px
    background linear-gradient(to right, rgba(0, 120, 212, 0.7), rgba(0, 120, 212, 0))

.drop-indicator-right
    right 0
    top 0
    bottom 0
    width 40px
    background linear-gradient(to left, rgba(0, 120, 212, 0.7), rgba(0, 120, 212, 0))

.drop-indicator-top
    top 0
    left 0
    right 0
    height 40px
    background linear-gradient(to bottom, rgba(0, 120, 212, 0.7), rgba(0, 120, 212, 0))

.drop-indicator-bottom
    bottom 0
    left 0
    right 0
    height 40px
    background linear-gradient(to top, rgba(0, 120, 212, 0.7), rgba(0, 120, 212, 0))

.terminal-header
    display flex
    align-items center
    padding 6px 10px
    background #3c3c3c
    border-bottom 1px solid #4a4a4a
    gap 8px

.terminal-drag-handle
    display flex
    align-items center
    justify-content center
    cursor grab
    color #666
    padding 4px
    margin -4px
    border-radius 4px
    transition all 0.15s ease
    user-select none

    &:hover
        color #999
        background rgba(255, 255, 255, 0.1)

    &:active
        cursor grabbing
        color #fff

    .icon
        width 16px
        height 16px
        pointer-events none

.terminal-title
    font-size 12px
    color #ccc
    font-family 'JetBrains Mono', 'Consolas', monospace
    white-space nowrap
    overflow hidden
    text-overflow ellipsis
    flex 1

.terminal-actions
    display flex
    gap 4px

.btn-add-terminal
    display flex
    align-items center
    justify-content center
    width 22px
    height 22px
    padding 0
    background none
    border none
    color #888
    cursor pointer
    border-radius 3px
    transition all 0.15s ease

    &:hover
        background rgba(255, 255, 255, 0.1)
        color #4caf50

    .icon
        width 14px
        height 14px

.btn-popout-header
    width 36px
    height 36px
    display flex
    align-items center
    justify-content center
    background-color #6c757d
    color white
    border none
    border-radius 6px
    cursor pointer
    transition all 0.2s ease

    &:hover
        background-color #5a6268

    .icon
        width 18px
        height 18px


.terminal-content
    flex 1
    overflow hidden
    min-height 200px

    :deep(.xterm)
        height 100%
        padding 4px

    :deep(.xterm-viewport)
        overflow-y auto !important
</style>

<style lang="stylus">
// Global styles for terminal dragging
body.terminal-dragging
    .terminal-content
        pointer-events none !important

    .xterm
        pointer-events none !important

    .sortable-chosen
        .terminal-content
            visibility hidden
</style>
