<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
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

const route = useRoute();
const workingDirectory = ref<string>((route.query.workingDir as string) || '');
const title = ref<string>((route.query.title as string) || 'Terminal');
const command = ref<string>((route.query.command as string) || '');
const terminalRef = ref<HTMLDivElement | null>(null);

let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let sessionId: string | null = null;
let outputUnlisten: UnlistenFn | null = null;
let resizeObserver: ResizeObserver | null = null;

async function updateWindowTitle(newTitle: string) {
    try {
        const win = getCurrentWindow();
        await win.setTitle(newTitle);
    } catch (error) {
        console.error('Failed to set window title:', error);
    }
}

onMounted(async () => {
    await updateWindowTitle(title.value);

    if (!terminalRef.value) return;

    // Create terminal
    terminal = new Terminal({
        cursorBlink: true,
        fontSize: 14,
        fontFamily: "'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
        theme: {
            background: '#1e1e1e',
            foreground: '#d4d4d4',
            cursor: '#ffffff',
            selectionBackground: '#264f78',
        },
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(terminalRef.value);

    setTimeout(() => fitAddon?.fit(), 100);

    // Handle input
    terminal.onData(async (data) => {
        if (sessionId) {
            try {
                await invoke('write_to_pty', { sessionId, data });
            } catch (error) {
                console.error('Failed to write to PTY:', error);
            }
        }
    });

    // Listen for PTY output
    outputUnlisten = await listen<PtyOutputEvent>('pty-output', (event) => {
        if (event.payload.session_id === sessionId && terminal) {
            terminal.write(event.payload.data);
        }
    });

    // Setup resize observer
    resizeObserver = new ResizeObserver(() => {
        if (fitAddon && sessionId && terminal) {
            fitAddon.fit();
            invoke('resize_pty', {
                sessionId,
                rows: terminal.rows,
                cols: terminal.cols,
            }).catch(console.error);
        }
    });
    resizeObserver.observe(terminalRef.value);

    // Create PTY session
    try {
        sessionId = await invoke<string>('create_pty_session', {
            rows: terminal.rows,
            cols: terminal.cols,
            workingDirectory: workingDirectory.value || null,
        });

        // Execute command if provided
        if (command.value) {
            await invoke('write_to_pty', {
                sessionId,
                data: command.value + '\n',
            });
        }
    } catch (error) {
        console.error('Failed to create PTY session:', error);
        terminal?.writeln(`Error: ${error}`);
    }

    terminal.focus();
});

onUnmounted(async () => {
    if (outputUnlisten) outputUnlisten();
    if (resizeObserver) resizeObserver.disconnect();

    // Kill PTY session
    if (sessionId) {
        try {
            await invoke('kill_pty_session', { sessionId });
        } catch (error) {
            console.error('Failed to kill PTY session:', error);
        }
    }

    if (terminal) {
        terminal.dispose();
    }
});
</script>

<template>
    <div class="terminal-popup">
        <div ref="terminalRef" class="terminal-container"></div>
    </div>
</template>

<style scoped lang="stylus">
.terminal-popup
    width 100vw
    height 100vh
    background #1e1e1e
    overflow hidden

.terminal-container
    width 100%
    height 100%

    :deep(.xterm)
        height 100%
        padding 8px

    :deep(.xterm-viewport)
        overflow-y auto !important
</style>
