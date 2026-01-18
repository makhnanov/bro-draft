<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import '@xterm/xterm/css/xterm.css';

interface PtyOutputEvent {
    session_id: string;
    data: string;
}

interface PtyExitEvent {
    session_id: string;
    exit_code: number | null;
}

const props = defineProps<{
    sessionId?: string;
    workingDirectory?: string;
    autoCreate?: boolean;
}>();

const emit = defineEmits<{
    (e: 'session-created', sessionId: string): void;
    (e: 'session-exit', sessionId: string, exitCode: number | null): void;
}>();

const terminalRef = ref<HTMLDivElement | null>(null);
const currentSessionId = ref<string | null>(props.sessionId || null);
const isReady = ref(false);

let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let outputUnlisten: UnlistenFn | null = null;
let exitUnlisten: UnlistenFn | null = null;
let resizeObserver: ResizeObserver | null = null;

async function createSession() {
    if (currentSessionId.value) return currentSessionId.value;

    try {
        // Get terminal dimensions
        const cols = terminal?.cols || 80;
        const rows = terminal?.rows || 24;

        const sessionId = await invoke<string>('create_pty_session', {
            rows,
            cols,
            workingDirectory: props.workingDirectory || null,
        });

        currentSessionId.value = sessionId;
        emit('session-created', sessionId);
        return sessionId;
    } catch (error) {
        console.error('Failed to create PTY session:', error);
        terminal?.writeln(`\r\nError: ${error}`);
        return null;
    }
}

async function sendInput(data: string) {
    if (!currentSessionId.value) return;

    try {
        await invoke('write_to_pty', {
            sessionId: currentSessionId.value,
            data,
        });
    } catch (error) {
        console.error('Failed to write to PTY:', error);
    }
}

async function resizeTerminal() {
    if (!terminal || !fitAddon || !currentSessionId.value) return;

    try {
        fitAddon.fit();
        await invoke('resize_pty', {
            sessionId: currentSessionId.value,
            rows: terminal.rows,
            cols: terminal.cols,
        });
    } catch (error) {
        console.error('Failed to resize PTY:', error);
    }
}

async function killSession() {
    if (!currentSessionId.value) return;

    try {
        await invoke('kill_pty_session', {
            sessionId: currentSessionId.value,
        });
    } catch (error) {
        console.error('Failed to kill PTY session:', error);
    }
}

onMounted(async () => {
    if (!terminalRef.value) return;

    // Initialize xterm.js
    terminal = new Terminal({
        cursorBlink: true,
        fontSize: 14,
        fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
        theme: {
            background: '#1e1e1e',
            foreground: '#d4d4d4',
            cursor: '#ffffff',
            cursorAccent: '#000000',
            selectionBackground: '#264f78',
            black: '#000000',
            red: '#cd3131',
            green: '#0dbc79',
            yellow: '#e5e510',
            blue: '#2472c8',
            magenta: '#bc3fbc',
            cyan: '#11a8cd',
            white: '#e5e5e5',
            brightBlack: '#666666',
            brightRed: '#f14c4c',
            brightGreen: '#23d18b',
            brightYellow: '#f5f543',
            brightBlue: '#3b8eea',
            brightMagenta: '#d670d6',
            brightCyan: '#29b8db',
            brightWhite: '#ffffff',
        },
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    terminal.open(terminalRef.value);
    fitAddon.fit();

    // Handle user input
    terminal.onData((data) => {
        sendInput(data);
    });

    // Listen for PTY output
    outputUnlisten = await listen<PtyOutputEvent>('pty-output', (event) => {
        if (event.payload.session_id === currentSessionId.value) {
            terminal?.write(event.payload.data);
        }
    });

    // Listen for PTY exit
    exitUnlisten = await listen<PtyExitEvent>('pty-exit', (event) => {
        if (event.payload.session_id === currentSessionId.value) {
            terminal?.writeln(`\r\nProcess exited with code: ${event.payload.exit_code ?? 'unknown'}`);
            emit('session-exit', event.payload.session_id, event.payload.exit_code);
        }
    });

    // Setup resize observer
    resizeObserver = new ResizeObserver(() => {
        resizeTerminal();
    });
    resizeObserver.observe(terminalRef.value);

    isReady.value = true;

    // Auto-create session if enabled
    if (props.autoCreate && !props.sessionId) {
        await createSession();
    }
});

onUnmounted(() => {
    if (outputUnlisten) outputUnlisten();
    if (exitUnlisten) exitUnlisten();
    if (resizeObserver) resizeObserver.disconnect();
    if (terminal) terminal.dispose();
});

// Watch for sessionId changes
watch(() => props.sessionId, (newSessionId) => {
    if (newSessionId) {
        currentSessionId.value = newSessionId;
    }
});

// Expose methods
defineExpose({
    createSession,
    killSession,
    resizeTerminal,
    getSessionId: () => currentSessionId.value,
    focus: () => terminal?.focus(),
});
</script>

<template>
    <div class="xterm-container" ref="terminalRef"></div>
</template>

<style scoped lang="stylus">
.xterm-container
    width 100%
    height 100%
    background #1e1e1e

    :deep(.xterm)
        height 100%
        padding 8px

    :deep(.xterm-viewport)
        overflow-y auto !important

    :deep(.xterm-screen)
        height 100%
</style>
