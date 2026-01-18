<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { useRoute } from 'vue-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
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
}

interface TerminalRow {
    id: string;
    commands: Command[];
}

const route = useRoute();
const projectId = ref<number>(parseInt(route.query.projectId as string) || 0);
const projectName = ref<string>((route.query.name as string) || 'Terminal');

const rows = ref<TerminalRow[]>([]);
let outputUnlisten: UnlistenFn | null = null;
let resizeObservers: Map<string, ResizeObserver> = new Map();

// Map sessionId -> Command for fast lookup
const sessionToCmd: Map<string, Command> = new Map();

// Drag state
const draggedTerminal = ref<{ rowId: string; cmdId: number } | null>(null);
const dropTarget = ref<{ rowId: string; cmdId: number; position: 'left' | 'right' | 'top' | 'bottom' } | null>(null);

async function updateWindowTitle() {
    try {
        const win = getCurrentWindow();
        await win.setTitle(projectName.value);
    } catch (error) {
        console.error('Failed to set window title:', error);
    }
}

function loadProjectCommands(): Command[] {
    console.log('Loading project commands for projectId:', projectId.value);
    const saved = localStorage.getItem('terminal_projects_v4');
    console.log('localStorage data:', saved);
    if (saved) {
        try {
            const projects = JSON.parse(saved);
            console.log('Parsed projects:', projects);
            const project = projects.find((p: any) => p.id === projectId.value);
            console.log('Found project:', project);
            if (project) {
                const commands = project.commands.map((c: any) => ({
                    id: c.id,
                    command: c.command,
                    workingDirectory: c.workingDirectory,
                    sessionId: null,
                    terminal: null,
                    fitAddon: null,
                }));
                console.log('Commands to create:', commands.length);
                return commands;
            }
        } catch (e) {
            console.error('Failed to load project:', e);
        }
    }
    console.log('No commands found, returning empty array');
    return [];
}

async function initTerminal(cmd: Command) {
    const containerId = `terminal-${cmd.id}`;
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
        sessionToCmd.set(sessionId, cmd);
        console.log('PTY session created for cmd:', cmd.id, 'sessionId:', sessionId);

        // Execute the command
        if (cmd.command) {
            console.log('Executing command:', cmd.command);
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

// Drag & Drop handlers
function startTerminalDrag(event: MouseEvent, row: TerminalRow, cmd: Command) {
    event.preventDefault();
    draggedTerminal.value = { rowId: row.id, cmdId: cmd.id };
    document.body.classList.add('terminal-dragging');

    document.addEventListener('mousemove', onTerminalDrag);
    document.addEventListener('mouseup', endTerminalDrag);
}

function onTerminalDrag(event: MouseEvent) {
    if (!draggedTerminal.value) return;

    const elementsUnder = document.elementsFromPoint(event.clientX, event.clientY);
    const terminalEl = elementsUnder.find(el => el.classList.contains('terminal-window')) as HTMLElement | undefined;

    if (terminalEl) {
        const cmdId = parseInt(terminalEl.dataset.cmdId || '0');
        const rowId = terminalEl.dataset.rowId || '';

        if (cmdId === draggedTerminal.value.cmdId) {
            dropTarget.value = null;
            return;
        }

        const rect = terminalEl.getBoundingClientRect();
        const relX = (event.clientX - rect.left) / rect.width;
        const relY = (event.clientY - rect.top) / rect.height;

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

        dropTarget.value = { rowId, cmdId, position };
    } else {
        dropTarget.value = null;
    }
}

function endTerminalDrag() {
    if (draggedTerminal.value && dropTarget.value) {
        performDrop();
    }

    draggedTerminal.value = null;
    dropTarget.value = null;
    document.body.classList.remove('terminal-dragging');

    document.removeEventListener('mousemove', onTerminalDrag);
    document.removeEventListener('mouseup', endTerminalDrag);
}

function performDrop() {
    if (!draggedTerminal.value || !dropTarget.value) return;

    const sourceRowIndex = rows.value.findIndex(r => r.id === draggedTerminal.value!.rowId);
    if (sourceRowIndex === -1) return;

    const sourceRow = rows.value[sourceRowIndex];
    const sourceCmdIndex = sourceRow.commands.findIndex(c => c.id === draggedTerminal.value!.cmdId);
    if (sourceCmdIndex === -1) return;

    const [movedCmd] = sourceRow.commands.splice(sourceCmdIndex, 1);

    const targetRowIndex = rows.value.findIndex(r => r.id === dropTarget.value!.rowId);
    if (targetRowIndex === -1) return;

    const targetRow = rows.value[targetRowIndex];
    const targetCmdIndex = targetRow.commands.findIndex(c => c.id === dropTarget.value!.cmdId);

    const position = dropTarget.value.position;

    if (position === 'left' || position === 'right') {
        const insertIndex = position === 'left' ? targetCmdIndex : targetCmdIndex + 1;
        targetRow.commands.splice(insertIndex, 0, movedCmd);
    } else {
        const newRow: TerminalRow = {
            id: `row-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
            commands: [movedCmd],
        };
        const insertRowIndex = position === 'top' ? targetRowIndex : targetRowIndex + 1;
        rows.value.splice(insertRowIndex, 0, newRow);
    }

    // Remove empty source row
    if (sourceRow.commands.length === 0) {
        const idx = rows.value.findIndex(r => r.id === sourceRow.id);
        if (idx !== -1) {
            rows.value.splice(idx, 1);
        }
    }

    // Recreate terminal on new DOM element
    nextTick(async () => {
        await recreateTerminal(movedCmd);

        // Re-fit all terminals
        for (const row of rows.value) {
            for (const cmd of row.commands) {
                if (cmd.fitAddon && cmd.terminal && cmd.id !== movedCmd.id) {
                    cmd.fitAddon.fit();
                }
            }
        }
    });
}

async function recreateTerminal(cmd: Command) {
    const containerId = `terminal-${cmd.id}`;
    const container = document.getElementById(containerId);
    if (!container) return;

    // Disconnect old ResizeObserver
    const oldObserver = resizeObservers.get(containerId);
    if (oldObserver) {
        oldObserver.disconnect();
        resizeObservers.delete(containerId);
    }

    // Save terminal buffer content
    let savedContent: string[] = [];
    if (cmd.terminal) {
        const buffer = cmd.terminal.buffer.active;
        for (let i = 0; i < buffer.length; i++) {
            const line = buffer.getLine(i);
            if (line) {
                savedContent.push(line.translateToString(true));
            }
        }
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

    if (savedContent.length > 0) {
        terminal.write(savedContent.join('\r\n') + '\r\n');
    }

    setTimeout(() => fitAddon.fit(), 50);

    cmd.terminal = terminal;
    cmd.fitAddon = fitAddon;

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

    if (cmd.sessionId) {
        await invoke('resize_pty', {
            sessionId: cmd.sessionId,
            rows: terminal.rows,
            cols: terminal.cols,
        }).catch(console.error);
    }

    terminal.focus();
}

function getDropPosition(rowId: string, cmdId: number): string | null {
    if (dropTarget.value?.rowId === rowId && dropTarget.value?.cmdId === cmdId) {
        return dropTarget.value.position;
    }
    return null;
}

function isDraggedTerminal(cmdId: number): boolean {
    return draggedTerminal.value?.cmdId === cmdId;
}

// Add new terminal to the right
async function addTerminalToRight(row: TerminalRow, afterCmd: Command) {
    const newCmd: Command = {
        id: Date.now(),
        command: '',
        workingDirectory: afterCmd.workingDirectory,
        sessionId: null,
        terminal: null,
        fitAddon: null,
    };

    const cmdIndex = row.commands.findIndex(c => c.id === afterCmd.id);
    row.commands.splice(cmdIndex + 1, 0, newCmd);

    await nextTick();
    await initTerminal(newCmd);

    for (const cmd of row.commands) {
        if (cmd.fitAddon) {
            cmd.fitAddon.fit();
        }
    }
}

onMounted(async () => {
    await updateWindowTitle();

    // Listen for PTY output FIRST (before creating terminals)
    outputUnlisten = await listen<PtyOutputEvent>('pty-output', (event) => {
        const cmd = sessionToCmd.get(event.payload.session_id);
        if (cmd && cmd.terminal) {
            cmd.terminal.write(event.payload.data);
        }
    });

    // Load commands and create initial rows
    const commands = loadProjectCommands();
    rows.value = commands.map(cmd => ({
        id: `row-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        commands: [cmd],
    }));

    await nextTick();

    // Initialize all terminals
    for (const row of rows.value) {
        for (const cmd of row.commands) {
            await initTerminal(cmd);
        }
    }
});

onUnmounted(async () => {
    if (outputUnlisten) outputUnlisten();

    // Kill all PTY sessions
    for (const row of rows.value) {
        for (const cmd of row.commands) {
            if (cmd.sessionId) {
                try {
                    await invoke('kill_pty_session', { sessionId: cmd.sessionId });
                } catch (error) {
                    console.error('Failed to kill PTY session:', error);
                }
            }
            if (cmd.terminal) {
                cmd.terminal.dispose();
            }
        }
    }

    resizeObservers.forEach(observer => observer.disconnect());
    sessionToCmd.clear();
});
</script>

<template>
    <div class="terminal-project-popup">
        <div class="terminals-grid">
            <div
                v-for="row in rows"
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
                            @mousedown="(e) => startTerminalDrag(e, row, cmd)"
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
                                @click="addTerminalToRight(row, cmd)"
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
                    <div :id="`terminal-${cmd.id}`" class="terminal-content"></div>
                    <!-- Drop zone indicators -->
                    <div class="drop-indicator drop-indicator-left"></div>
                    <div class="drop-indicator drop-indicator-right"></div>
                    <div class="drop-indicator drop-indicator-top"></div>
                    <div class="drop-indicator drop-indicator-bottom"></div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped lang="stylus">
.terminal-project-popup
    width 100vw
    height 100vh
    background #2d2d30
    overflow hidden

.terminals-grid
    width 100%
    height 100%
    display flex
    flex-direction column
    gap 4px
    padding 4px

.terminal-row
    display flex
    gap 4px
    flex 1
    min-height 0

.terminal-window
    display flex
    flex-direction column
    background #1e1e1e
    border-radius 4px
    overflow hidden
    min-width 200px
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
    padding 4px 8px
    background #3c3c3c
    border-bottom 1px solid #4a4a4a
    gap 8px
    flex-shrink 0

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
    width 20px
    height 20px
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

.terminal-content
    flex 1
    overflow hidden
    min-height 0

    :deep(.xterm)
        height 100%
        padding 4px

    :deep(.xterm-viewport)
        overflow-y auto !important

.icon
    width 16px
    height 16px
</style>

<style lang="stylus">
body.terminal-dragging
    .terminal-content
        pointer-events none !important

    .xterm
        pointer-events none !important
</style>
