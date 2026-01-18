<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { useRoute } from 'vue-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SerializeAddon } from '@xterm/addon-serialize';
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
    serializeAddon: SerializeAddon | null;
}

// Nested layout structure
interface LayoutNode {
    id: string;
    type: 'terminal' | 'container';
    command?: Command;
    direction?: 'horizontal' | 'vertical';
    children?: LayoutNode[];
}

const route = useRoute();
const projectId = ref<number>(parseInt(route.query.projectId as string) || 0);
const projectName = ref<string>((route.query.name as string) || 'Terminal');

const layout = ref<LayoutNode | null>(null);
let outputUnlisten: UnlistenFn | null = null;
let resizeObservers: Map<string, ResizeObserver> = new Map();
const sessionToCmd: Map<string, Command> = new Map();

// Drag state
const draggedNodeId = ref<string | null>(null);
const dropTarget = ref<{
    type: 'terminal' | 'edge';
    nodeId?: string;
    position: 'left' | 'right' | 'top' | 'bottom';
} | null>(null);

async function updateWindowTitle() {
    try {
        const win = getCurrentWindow();
        await win.setTitle(projectName.value);
    } catch (error) {
        console.error('Failed to set window title:', error);
    }
}

function loadProjectCommands(): Command[] {
    const saved = localStorage.getItem('terminal_projects_v4');
    if (saved) {
        try {
            const projects = JSON.parse(saved);
            const project = projects.find((p: any) => p.id === projectId.value);
            if (project) {
                return project.commands.map((c: any) => ({
                    id: c.id,
                    command: c.command,
                    workingDirectory: c.workingDirectory,
                    sessionId: null,
                    terminal: null,
                    fitAddon: null,
                    serializeAddon: null,
                }));
            }
        } catch (e) {
            console.error('Failed to load project:', e);
        }
    }
    return [];
}

function createInitialLayout(commands: Command[]): LayoutNode | null {
    if (commands.length === 0) return null;
    if (commands.length === 1) {
        return {
            id: `node-${commands[0].id}`,
            type: 'terminal',
            command: commands[0],
        };
    }
    // Multiple commands - create horizontal container
    return {
        id: `container-${Date.now()}`,
        type: 'container',
        direction: 'horizontal',
        children: commands.map(cmd => ({
            id: `node-${cmd.id}`,
            type: 'terminal',
            command: cmd,
        })),
    };
}

function findNodeById(node: LayoutNode | null, id: string): LayoutNode | null {
    if (!node) return null;
    if (node.id === id) return node;
    if (node.children) {
        for (const child of node.children) {
            const found = findNodeById(child, id);
            if (found) return found;
        }
    }
    return null;
}

function findParentNode(root: LayoutNode | null, nodeId: string): { parent: LayoutNode; index: number } | null {
    if (!root || root.type !== 'container' || !root.children) return null;

    for (let i = 0; i < root.children.length; i++) {
        if (root.children[i].id === nodeId) {
            return { parent: root, index: i };
        }
        const found = findParentNode(root.children[i], nodeId);
        if (found) return found;
    }
    return null;
}

function removeNode(root: LayoutNode, nodeId: string): LayoutNode | null {
    if (root.id === nodeId) return null;

    if (root.type === 'container' && root.children) {
        root.children = root.children.filter(child => {
            if (child.id === nodeId) return false;
            if (child.type === 'container') {
                const result = removeNode(child, nodeId);
                if (!result) return false;
                Object.assign(child, result);
            }
            return true;
        });

        // Simplify: if only one child left, replace container with child
        if (root.children.length === 1) {
            return root.children[0];
        }
        if (root.children.length === 0) {
            return null;
        }
    }
    return root;
}

function getAllTerminalNodes(node: LayoutNode | null): LayoutNode[] {
    if (!node) return [];
    if (node.type === 'terminal') return [node];
    if (node.children) {
        return node.children.flatMap(child => getAllTerminalNodes(child));
    }
    return [];
}

// Save command references before layout modification
function saveCommandRefs(node: LayoutNode | null): Map<number, Command> {
    const refs = new Map<number, Command>();
    const terminals = getAllTerminalNodes(node);
    for (const t of terminals) {
        if (t.command) {
            refs.set(t.command.id, t.command);
        }
    }
    return refs;
}

// Restore command references after layout modification
function restoreCommandRefs(node: LayoutNode | null, refs: Map<number, Command>) {
    console.log('restoreCommandRefs called:', { node: node?.id, type: node?.type, hasChildren: !!node?.children });
    if (!node) return;
    if (node.type === 'terminal' && node.command) {
        const saved = refs.get(node.command.id);
        console.log('restoreCommandRefs terminal:', { cmdId: node.command.id, found: !!saved, savedSessionId: saved?.sessionId });
        if (saved) {
            node.command.terminal = saved.terminal;
            node.command.fitAddon = saved.fitAddon;
            node.command.serializeAddon = saved.serializeAddon;
            node.command.sessionId = saved.sessionId;
        }
    }
    if (node.children) {
        for (const child of node.children) {
            restoreCommandRefs(child, refs);
        }
    }
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
    const serializeAddon = new SerializeAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(serializeAddon);
    terminal.open(container);

    setTimeout(() => fitAddon.fit(), 100);

    cmd.terminal = terminal;
    cmd.fitAddon = fitAddon;
    cmd.serializeAddon = serializeAddon;

    terminal.onData(async (data) => {
        console.log('initTerminal onData:', { cmdId: cmd.id, sessionId: cmd.sessionId });
        if (cmd.sessionId) {
            try {
                await invoke('write_to_pty', { sessionId: cmd.sessionId, data });
            } catch (error) {
                console.error('Failed to write to PTY:', error);
            }
        } else {
            console.warn('initTerminal: No sessionId for terminal:', cmd.id);
        }
    });

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

    try {
        const sessionId = await invoke<string>('create_pty_session', {
            rows: terminal.rows,
            cols: terminal.cols,
            workingDirectory: cmd.workingDirectory || null,
        });
        cmd.sessionId = sessionId;
        sessionToCmd.set(sessionId, cmd);

        if (cmd.command) {
            await invoke('write_to_pty', { sessionId, data: cmd.command + '\n' });
        }
    } catch (error) {
        console.error('Failed to create PTY session:', error);
        terminal.writeln(`Error: ${error}`);
    }
}

// Drag & Drop
function startDrag(event: MouseEvent, node: LayoutNode) {
    if (node.type !== 'terminal') return;
    event.preventDefault();
    draggedNodeId.value = node.id;
    document.body.classList.add('terminal-dragging');
    document.addEventListener('mousemove', onDrag);
    document.addEventListener('mouseup', endDrag);
}

function onDrag(event: MouseEvent) {
    if (!draggedNodeId.value) return;

    const container = document.querySelector('.layout-root') as HTMLElement;
    if (!container) return;

    const containerRect = container.getBoundingClientRect();
    const edgeThreshold = 50;

    // Check container edges first
    const nearLeft = event.clientX - containerRect.left < edgeThreshold;
    const nearRight = containerRect.right - event.clientX < edgeThreshold;
    const nearTop = event.clientY - containerRect.top < edgeThreshold;
    const nearBottom = containerRect.bottom - event.clientY < edgeThreshold;

    if (nearLeft) {
        dropTarget.value = { type: 'edge', position: 'left' };
        return;
    }
    if (nearRight) {
        dropTarget.value = { type: 'edge', position: 'right' };
        return;
    }
    if (nearTop) {
        dropTarget.value = { type: 'edge', position: 'top' };
        return;
    }
    if (nearBottom) {
        dropTarget.value = { type: 'edge', position: 'bottom' };
        return;
    }

    // Check terminals
    const elements = document.elementsFromPoint(event.clientX, event.clientY);
    const terminalEl = elements.find(el => el.classList.contains('terminal-window')) as HTMLElement | undefined;

    if (terminalEl) {
        const nodeId = terminalEl.dataset.nodeId || '';
        if (nodeId === draggedNodeId.value) {
            dropTarget.value = null;
            return;
        }

        const rect = terminalEl.getBoundingClientRect();
        const relX = (event.clientX - rect.left) / rect.width;
        const relY = (event.clientY - rect.top) / rect.height;

        let position: 'left' | 'right' | 'top' | 'bottom';
        if (relX < 0.25) position = 'left';
        else if (relX > 0.75) position = 'right';
        else if (relY < 0.5) position = 'top';
        else position = 'bottom';

        dropTarget.value = { type: 'terminal', nodeId, position };
    } else {
        dropTarget.value = null;
    }
}

function endDrag() {
    console.log('endDrag called', { draggedNodeId: draggedNodeId.value, dropTarget: dropTarget.value });
    if (draggedNodeId.value && dropTarget.value) {
        try {
            performDrop();
        } catch (e) {
            console.error('performDrop error:', e);
        }
    }
    draggedNodeId.value = null;
    dropTarget.value = null;
    document.body.classList.remove('terminal-dragging');
    document.removeEventListener('mousemove', onDrag);
    document.removeEventListener('mouseup', endDrag);
}

function performDrop() {
    console.log('performDrop called');
    if (!draggedNodeId.value || !dropTarget.value || !layout.value) {
        console.log('performDrop early return', { draggedNodeId: draggedNodeId.value, dropTarget: dropTarget.value, layout: layout.value });
        return;
    }

    // Save all command references before modification
    const commandRefs = saveCommandRefs(layout.value);

    const draggedNode = findNodeById(layout.value, draggedNodeId.value);
    console.log('draggedNode:', draggedNode);
    if (!draggedNode || draggedNode.type !== 'terminal') {
        console.log('draggedNode not found or not terminal');
        return;
    }

    // Remove dragged node from layout
    const layoutCopy = JSON.parse(JSON.stringify(layout.value, (key, value) => {
        if (key === 'terminal' || key === 'fitAddon' || key === 'serializeAddon') return undefined;
        return value;
    }));
    const newLayout = removeNode(layoutCopy, draggedNodeId.value);
    console.log('newLayout after remove:', newLayout);

    // Clone the dragged node for insertion
    const movedNode: LayoutNode = {
        id: draggedNode.id,
        type: 'terminal',
        command: draggedNode.command,
    };

    if (dropTarget.value.type === 'edge') {
        // Drop on container edge
        const position = dropTarget.value.position;
        const direction = (position === 'left' || position === 'right') ? 'horizontal' : 'vertical';
        const insertFirst = position === 'left' || position === 'top';

        if (!newLayout) {
            layout.value = movedNode;
        } else if (newLayout.type === 'container' && newLayout.direction === direction) {
            // Same direction - just add to children
            if (insertFirst) {
                newLayout.children!.unshift(movedNode);
            } else {
                newLayout.children!.push(movedNode);
            }
            layout.value = newLayout;
        } else {
            // Different direction or terminal - wrap in new container
            layout.value = {
                id: `container-${Date.now()}`,
                type: 'container',
                direction,
                children: insertFirst ? [movedNode, newLayout] : [newLayout, movedNode],
            };
        }
    } else {
        // Drop on terminal
        const targetNode = findNodeById(newLayout, dropTarget.value.nodeId!);
        if (!targetNode) return;

        const position = dropTarget.value.position;
        const direction = (position === 'left' || position === 'right') ? 'horizontal' : 'vertical';
        const insertFirst = position === 'left' || position === 'top';

        // Find parent of target
        const parentInfo = findParentNode(newLayout!, dropTarget.value.nodeId!);

        if (parentInfo && parentInfo.parent.direction === direction) {
            // Same direction - insert next to target
            const insertIndex = insertFirst ? parentInfo.index : parentInfo.index + 1;
            parentInfo.parent.children!.splice(insertIndex, 0, movedNode);
            layout.value = newLayout;
        } else {
            // Different direction - wrap target in new container
            const newContainer: LayoutNode = {
                id: `container-${Date.now()}`,
                type: 'container',
                direction,
                children: insertFirst ? [movedNode, { ...targetNode }] : [{ ...targetNode }, movedNode],
            };

            if (parentInfo) {
                parentInfo.parent.children![parentInfo.index] = newContainer;
                layout.value = newLayout;
            } else {
                // Target is root
                layout.value = newContainer;
            }
        }
    }

    // Restore command references for all terminals
    console.log('About to call restoreCommandRefs, layout.value:', JSON.stringify(layout.value, (k,v) => ['terminal','fitAddon','serializeAddon'].includes(k) ? '[object]' : v));
    console.log('commandRefs size:', commandRefs.size, 'keys:', [...commandRefs.keys()]);
    restoreCommandRefs(layout.value, commandRefs);

    nextTick(async () => {
        // Recreate ALL terminals since layout structure changed
        const allTerminals = getAllTerminalNodes(layout.value);
        for (const node of allTerminals) {
            if (node.command) {
                await recreateTerminal(node.command);
            }
        }
        refitAllTerminals();
    });
}

async function recreateTerminal(cmd: Command) {
    const containerId = `terminal-${cmd.id}`;
    const container = document.getElementById(containerId);
    if (!container) return;

    const oldObserver = resizeObservers.get(containerId);
    if (oldObserver) {
        oldObserver.disconnect();
        resizeObservers.delete(containerId);
    }

    // Save content with colors using serializeAddon
    let savedContent = '';
    if (cmd.terminal && cmd.serializeAddon) {
        savedContent = cmd.serializeAddon.serialize();
        cmd.terminal.dispose();
    } else if (cmd.terminal) {
        // Fallback without colors
        const lines: string[] = [];
        const buffer = cmd.terminal.buffer.active;
        for (let i = 0; i < buffer.length; i++) {
            const line = buffer.getLine(i);
            if (line) lines.push(line.translateToString(true));
        }
        while (lines.length > 0 && lines[lines.length - 1].trim() === '') {
            lines.pop();
        }
        savedContent = lines.join('\r\n') + '\r\n';
        cmd.terminal.dispose();
    }

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
    const serializeAddon = new SerializeAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(serializeAddon);
    terminal.open(container);

    if (savedContent) {
        terminal.write(savedContent);
    }

    setTimeout(() => fitAddon.fit(), 50);
    cmd.terminal = terminal;
    cmd.fitAddon = fitAddon;
    cmd.serializeAddon = serializeAddon;

    console.log('recreateTerminal done:', { cmdId: cmd.id, sessionId: cmd.sessionId });

    // Update sessionToCmd map to point to current Command object
    if (cmd.sessionId) {
        sessionToCmd.set(cmd.sessionId, cmd);
    }

    terminal.onData(async (data) => {
        console.log('recreateTerminal onData:', { cmdId: cmd.id, sessionId: cmd.sessionId });
        if (cmd.sessionId) {
            try {
                await invoke('write_to_pty', { sessionId: cmd.sessionId, data });
            } catch (error) {
                console.error('Failed to write to PTY:', error);
            }
        } else {
            console.warn('recreateTerminal: No sessionId for terminal:', cmd.id);
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

function refitAllTerminals() {
    const terminals = getAllTerminalNodes(layout.value);
    for (const node of terminals) {
        if (node.command?.fitAddon) {
            node.command.fitAddon.fit();
        }
    }
}

function getTerminalDropPosition(nodeId: string): string | null {
    if (dropTarget.value?.type === 'terminal' && dropTarget.value?.nodeId === nodeId) {
        return dropTarget.value.position;
    }
    return null;
}

function isDragging(nodeId: string): boolean {
    return draggedNodeId.value === nodeId;
}

function getEdgeDropPosition(): string | null {
    if (dropTarget.value?.type === 'edge') {
        return dropTarget.value.position;
    }
    return null;
}

async function addTerminal(afterNode: LayoutNode) {
    if (!afterNode.command || !layout.value) return;

    // Save command refs before modification
    const commandRefs = saveCommandRefs(layout.value);
    const existingCmd = afterNode.command;

    const newCmd: Command = {
        id: Date.now(),
        command: '',
        workingDirectory: afterNode.command.workingDirectory,
        sessionId: null,
        terminal: null,
        fitAddon: null,
        serializeAddon: null,
    };

    const newNode: LayoutNode = {
        id: `node-${newCmd.id}`,
        type: 'terminal',
        command: newCmd,
    };

    let needRecreateExisting = false;

    const parentInfo = findParentNode(layout.value, afterNode.id);
    if (parentInfo && parentInfo.parent.direction === 'horizontal') {
        // Parent is horizontal - insert directly (no DOM change for existing)
        parentInfo.parent.children!.splice(parentInfo.index + 1, 0, newNode);
    } else if (parentInfo) {
        // Parent is vertical - wrap current terminal in horizontal container with new one
        const newContainer: LayoutNode = {
            id: `container-${Date.now()}`,
            type: 'container',
            direction: 'horizontal',
            children: [{ ...afterNode }, newNode],
        };
        parentInfo.parent.children![parentInfo.index] = newContainer;
        needRecreateExisting = true; // DOM element will be recreated
    } else {
        // afterNode is root - wrap in horizontal container
        layout.value = {
            id: `container-${Date.now()}`,
            type: 'container',
            direction: 'horizontal',
            children: [afterNode, newNode],
        };
        needRecreateExisting = true; // DOM element will be recreated
    }

    // Restore command refs for existing terminals
    restoreCommandRefs(layout.value, commandRefs);

    await nextTick();

    // Recreate existing terminal if its DOM element was recreated
    if (needRecreateExisting) {
        await recreateTerminal(existingCmd);
    }

    await initTerminal(newCmd);
    refitAllTerminals();
}

onMounted(async () => {
    await updateWindowTitle();

    outputUnlisten = await listen<PtyOutputEvent>('pty-output', (event) => {
        const cmd = sessionToCmd.get(event.payload.session_id);
        if (cmd && cmd.terminal) {
            cmd.terminal.write(event.payload.data);
        }
    });

    const commands = loadProjectCommands();
    layout.value = createInitialLayout(commands);

    await nextTick();

    const terminals = getAllTerminalNodes(layout.value);
    for (const node of terminals) {
        if (node.command) {
            await initTerminal(node.command);
        }
    }
});

onUnmounted(async () => {
    if (outputUnlisten) outputUnlisten();

    const terminals = getAllTerminalNodes(layout.value);
    for (const node of terminals) {
        if (node.command) {
            if (node.command.sessionId) {
                try {
                    await invoke('kill_pty_session', { sessionId: node.command.sessionId });
                } catch (error) {
                    console.error('Failed to kill PTY session:', error);
                }
            }
            if (node.command.terminal) {
                node.command.terminal.dispose();
            }
        }
    }

    resizeObservers.forEach(observer => observer.disconnect());
    sessionToCmd.clear();
});
</script>

<template>
    <div class="terminal-project-popup">
        <div
            class="layout-root"
            :class="{
                'edge-drop-left': getEdgeDropPosition() === 'left',
                'edge-drop-right': getEdgeDropPosition() === 'right',
                'edge-drop-top': getEdgeDropPosition() === 'top',
                'edge-drop-bottom': getEdgeDropPosition() === 'bottom',
            }"
        >
            <!-- Recursive layout rendering -->
            <template v-if="layout">
                <component
                    :is="'div'"
                    v-if="layout.type === 'container'"
                    class="layout-container"
                    :class="[`direction-${layout.direction}`]"
                >
                    <template v-for="child in layout.children" :key="child.id">
                        <!-- Nested container -->
                        <div
                            v-if="child.type === 'container'"
                            class="layout-container"
                            :class="[`direction-${child.direction}`]"
                        >
                            <template v-for="grandchild in child.children" :key="grandchild.id">
                                <div
                                    v-if="grandchild.type === 'terminal' && grandchild.command"
                                    class="terminal-window"
                                    :class="{
                                        'is-dragging': isDragging(grandchild.id),
                                        'drop-left': getTerminalDropPosition(grandchild.id) === 'left',
                                        'drop-right': getTerminalDropPosition(grandchild.id) === 'right',
                                        'drop-top': getTerminalDropPosition(grandchild.id) === 'top',
                                        'drop-bottom': getTerminalDropPosition(grandchild.id) === 'bottom',
                                    }"
                                    :data-node-id="grandchild.id"
                                >
                                    <div class="terminal-header">
                                        <div class="terminal-drag-handle" @mousedown="(e) => startDrag(e, grandchild)">
                                            <svg viewBox="0 0 24 24" class="icon">
                                                <circle cx="5" cy="9" r="1.5" fill="currentColor"/>
                                                <circle cx="12" cy="9" r="1.5" fill="currentColor"/>
                                                <circle cx="19" cy="9" r="1.5" fill="currentColor"/>
                                                <circle cx="5" cy="15" r="1.5" fill="currentColor"/>
                                                <circle cx="12" cy="15" r="1.5" fill="currentColor"/>
                                                <circle cx="19" cy="15" r="1.5" fill="currentColor"/>
                                            </svg>
                                        </div>
                                        <span class="terminal-title">{{ grandchild.command.command || 'Terminal' }}</span>
                                        <button @click="addTerminal(grandchild)" class="btn-add" title="Add terminal">+</button>
                                    </div>
                                    <div :id="`terminal-${grandchild.command.id}`" class="terminal-content"></div>
                                    <div class="drop-indicator drop-indicator-left"></div>
                                    <div class="drop-indicator drop-indicator-right"></div>
                                    <div class="drop-indicator drop-indicator-top"></div>
                                    <div class="drop-indicator drop-indicator-bottom"></div>
                                </div>
                                <!-- Deeper nesting would go here if needed -->
                            </template>
                        </div>
                        <!-- Direct terminal child -->
                        <div
                            v-else-if="child.type === 'terminal' && child.command"
                            class="terminal-window"
                            :class="{
                                'is-dragging': isDragging(child.id),
                                'drop-left': getTerminalDropPosition(child.id) === 'left',
                                'drop-right': getTerminalDropPosition(child.id) === 'right',
                                'drop-top': getTerminalDropPosition(child.id) === 'top',
                                'drop-bottom': getTerminalDropPosition(child.id) === 'bottom',
                            }"
                            :data-node-id="child.id"
                        >
                            <div class="terminal-header">
                                <div class="terminal-drag-handle" @mousedown="(e) => startDrag(e, child)">
                                    <svg viewBox="0 0 24 24" class="icon">
                                        <circle cx="5" cy="9" r="1.5" fill="currentColor"/>
                                        <circle cx="12" cy="9" r="1.5" fill="currentColor"/>
                                        <circle cx="19" cy="9" r="1.5" fill="currentColor"/>
                                        <circle cx="5" cy="15" r="1.5" fill="currentColor"/>
                                        <circle cx="12" cy="15" r="1.5" fill="currentColor"/>
                                        <circle cx="19" cy="15" r="1.5" fill="currentColor"/>
                                    </svg>
                                </div>
                                <span class="terminal-title">{{ child.command.command || 'Terminal' }}</span>
                                <button @click="addTerminal(child)" class="btn-add" title="Add terminal">+</button>
                            </div>
                            <div :id="`terminal-${child.command.id}`" class="terminal-content"></div>
                            <div class="drop-indicator drop-indicator-left"></div>
                            <div class="drop-indicator drop-indicator-right"></div>
                            <div class="drop-indicator drop-indicator-top"></div>
                            <div class="drop-indicator drop-indicator-bottom"></div>
                        </div>
                    </template>
                </component>
                <!-- Single terminal (no container) -->
                <div
                    v-else-if="layout.type === 'terminal' && layout.command"
                    class="terminal-window single"
                    :class="{
                        'is-dragging': isDragging(layout.id),
                        'drop-left': getTerminalDropPosition(layout.id) === 'left',
                        'drop-right': getTerminalDropPosition(layout.id) === 'right',
                        'drop-top': getTerminalDropPosition(layout.id) === 'top',
                        'drop-bottom': getTerminalDropPosition(layout.id) === 'bottom',
                    }"
                    :data-node-id="layout.id"
                >
                    <div class="terminal-header">
                        <div class="terminal-drag-handle" @mousedown="(e) => startDrag(e, layout!)">
                            <svg viewBox="0 0 24 24" class="icon">
                                <circle cx="5" cy="9" r="1.5" fill="currentColor"/>
                                <circle cx="12" cy="9" r="1.5" fill="currentColor"/>
                                <circle cx="19" cy="9" r="1.5" fill="currentColor"/>
                                <circle cx="5" cy="15" r="1.5" fill="currentColor"/>
                                <circle cx="12" cy="15" r="1.5" fill="currentColor"/>
                                <circle cx="19" cy="15" r="1.5" fill="currentColor"/>
                            </svg>
                        </div>
                        <span class="terminal-title">{{ layout.command.command || 'Terminal' }}</span>
                        <button @click="addTerminal(layout!)" class="btn-add" title="Add terminal">+</button>
                    </div>
                    <div :id="`terminal-${layout.command.id}`" class="terminal-content"></div>
                    <div class="drop-indicator drop-indicator-left"></div>
                    <div class="drop-indicator drop-indicator-right"></div>
                    <div class="drop-indicator drop-indicator-top"></div>
                    <div class="drop-indicator drop-indicator-bottom"></div>
                </div>
            </template>
            <!-- Edge drop indicators -->
            <div class="edge-indicator edge-indicator-left"></div>
            <div class="edge-indicator edge-indicator-right"></div>
            <div class="edge-indicator edge-indicator-top"></div>
            <div class="edge-indicator edge-indicator-bottom"></div>
        </div>
    </div>
</template>

<style scoped lang="stylus">
.terminal-project-popup
    width 100vw
    height 100vh
    background #2d2d30
    overflow hidden

.layout-root
    width 100%
    height 100%
    padding 4px
    position relative
    display flex

    &.edge-drop-left .edge-indicator-left,
    &.edge-drop-right .edge-indicator-right,
    &.edge-drop-top .edge-indicator-top,
    &.edge-drop-bottom .edge-indicator-bottom
        opacity 1

.layout-container
    display flex
    gap 4px
    flex 1
    min-width 0
    min-height 0

    &.direction-horizontal
        flex-direction row

    &.direction-vertical
        flex-direction column

.terminal-window
    display flex
    flex-direction column
    background #1e1e1e
    border-radius 4px
    overflow hidden
    flex 1
    min-width 150px
    min-height 100px
    position relative

    &.single
        width 100%
        height 100%

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
    transition opacity 0.15s ease
    pointer-events none
    z-index 10

.drop-indicator-left
    left 0
    top 0
    bottom 0
    width 30px
    background linear-gradient(to right, rgba(0, 120, 212, 0.7), transparent)

.drop-indicator-right
    right 0
    top 0
    bottom 0
    width 30px
    background linear-gradient(to left, rgba(0, 120, 212, 0.7), transparent)

.drop-indicator-top
    top 0
    left 0
    right 0
    height 30px
    background linear-gradient(to bottom, rgba(0, 120, 212, 0.7), transparent)

.drop-indicator-bottom
    bottom 0
    left 0
    right 0
    height 30px
    background linear-gradient(to top, rgba(0, 120, 212, 0.7), transparent)

.edge-indicator
    position absolute
    opacity 0
    transition opacity 0.15s ease
    pointer-events none
    z-index 20
    background rgba(76, 175, 80, 0.6)

.edge-indicator-left
    left 0
    top 0
    bottom 0
    width 50px

.edge-indicator-right
    right 0
    top 0
    bottom 0
    width 50px

.edge-indicator-top
    top 0
    left 0
    right 0
    height 50px

.edge-indicator-bottom
    bottom 0
    left 0
    right 0
    height 50px

.terminal-header
    display flex
    align-items center
    padding 4px 8px
    background #3c3c3c
    border-bottom 1px solid #4a4a4a
    gap 8px
    flex-shrink 0

.terminal-drag-handle
    cursor grab
    color #666
    padding 2px
    border-radius 3px
    display flex
    align-items center
    user-select none

    &:hover
        color #999
        background rgba(255, 255, 255, 0.1)

    &:active
        cursor grabbing

    .icon
        width 16px
        height 16px
        pointer-events none

.terminal-title
    font-size 12px
    color #ccc
    font-family 'JetBrains Mono', monospace
    flex 1
    overflow hidden
    text-overflow ellipsis
    white-space nowrap

.btn-add
    background none
    border none
    color #888
    cursor pointer
    font-size 16px
    padding 0 4px
    border-radius 3px

    &:hover
        background rgba(255, 255, 255, 0.1)
        color #4caf50

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
    .terminal-content, .xterm
        pointer-events none !important
</style>
