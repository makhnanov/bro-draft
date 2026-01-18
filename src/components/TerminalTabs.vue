<script setup lang="ts">
import { ref, computed, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import XTerminal from './XTerminal.vue';

interface TerminalTab {
    id: string;
    title: string;
    sessionId: string | null;
    workingDirectory: string;
}

const props = defineProps<{
    workingDirectory?: string;
    initialTabs?: TerminalTab[];
}>();

const emit = defineEmits<{
    (e: 'tabs-change', tabs: TerminalTab[]): void;
    (e: 'popout', tab: TerminalTab): void;
}>();

const tabs = ref<TerminalTab[]>(props.initialTabs || []);
const activeTabId = ref<string | null>(null);
const terminalRefs = ref<Record<string, InstanceType<typeof XTerminal>>>({});

const activeTab = computed(() => {
    return tabs.value.find(t => t.id === activeTabId.value) || null;
});

function generateId(): string {
    return `tab-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

function addTab() {
    const newTab: TerminalTab = {
        id: generateId(),
        title: `Terminal ${tabs.value.length + 1}`,
        sessionId: null,
        workingDirectory: props.workingDirectory || '',
    };
    tabs.value.push(newTab);
    activeTabId.value = newTab.id;
    emit('tabs-change', tabs.value);

    // Focus new terminal after render
    nextTick(() => {
        const terminal = terminalRefs.value[newTab.id];
        if (terminal) {
            terminal.focus();
        }
    });
}

async function closeTab(tabId: string) {
    const tabIndex = tabs.value.findIndex(t => t.id === tabId);
    if (tabIndex === -1) return;

    const tab = tabs.value[tabIndex];

    // Kill PTY session if exists
    if (tab.sessionId) {
        try {
            await invoke('kill_pty_session', { sessionId: tab.sessionId });
        } catch (error) {
            console.error('Failed to kill PTY session:', error);
        }
    }

    tabs.value.splice(tabIndex, 1);

    // Switch to adjacent tab
    if (activeTabId.value === tabId) {
        if (tabs.value.length > 0) {
            const newIndex = Math.min(tabIndex, tabs.value.length - 1);
            activeTabId.value = tabs.value[newIndex].id;
        } else {
            activeTabId.value = null;
        }
    }

    emit('tabs-change', tabs.value);
}

function selectTab(tabId: string) {
    activeTabId.value = tabId;
    nextTick(() => {
        const terminal = terminalRefs.value[tabId];
        if (terminal) {
            terminal.focus();
        }
    });
}

function onSessionCreated(tabId: string, sessionId: string) {
    const tab = tabs.value.find(t => t.id === tabId);
    if (tab) {
        tab.sessionId = sessionId;
        emit('tabs-change', tabs.value);
    }
}

function popoutTab(tab: TerminalTab) {
    emit('popout', tab);
}

function setTerminalRef(tabId: string, el: any) {
    if (el) {
        terminalRefs.value[tabId] = el;
    } else {
        delete terminalRefs.value[tabId];
    }
}

// Initialize with one tab if empty
if (tabs.value.length === 0) {
    addTab();
}

// Expose methods
defineExpose({
    addTab,
    closeTab,
    getTabs: () => tabs.value,
    getActiveTab: () => activeTab.value,
});
</script>

<template>
    <div class="terminal-tabs">
        <div class="tabs-header">
            <div class="tabs-list">
                <div
                    v-for="tab in tabs"
                    :key="tab.id"
                    class="tab"
                    :class="{ active: tab.id === activeTabId }"
                    @click="selectTab(tab.id)"
                >
                    <span class="tab-title">{{ tab.title }}</span>
                    <button
                        class="tab-close"
                        @click.stop="closeTab(tab.id)"
                        title="Close"
                    >
                        <svg viewBox="0 0 24 24" class="icon">
                            <line x1="18" y1="6" x2="6" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            <line x1="6" y1="6" x2="18" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                        </svg>
                    </button>
                </div>
            </div>
            <div class="tabs-actions">
                <button class="btn-add-tab" @click="addTab" title="New Terminal">
                    <svg viewBox="0 0 24 24" class="icon">
                        <line x1="12" y1="5" x2="12" y2="19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                        <line x1="5" y1="12" x2="19" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    </svg>
                </button>
                <button
                    v-if="activeTab"
                    class="btn-popout"
                    @click="popoutTab(activeTab)"
                    title="Open in new window"
                >
                    <svg viewBox="0 0 24 24" class="icon">
                        <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
                        <polyline points="15 3 21 3 21 9" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
                        <line x1="10" y1="14" x2="21" y2="3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                </button>
            </div>
        </div>
        <div class="tabs-content">
            <div
                v-for="tab in tabs"
                :key="tab.id"
                class="tab-panel"
                :class="{ active: tab.id === activeTabId }"
            >
                <XTerminal
                    :ref="(el) => setTerminalRef(tab.id, el)"
                    :working-directory="tab.workingDirectory"
                    :auto-create="true"
                    @session-created="(sessionId) => onSessionCreated(tab.id, sessionId)"
                />
            </div>
        </div>
    </div>
</template>

<style scoped lang="stylus">
.terminal-tabs
    display flex
    flex-direction column
    height 100%
    background #1e1e1e
    border-radius 6px
    overflow hidden

.tabs-header
    display flex
    align-items center
    justify-content space-between
    background #2d2d30
    border-bottom 1px solid #3e3e42
    min-height 36px

.tabs-list
    display flex
    flex 1
    overflow-x auto
    scrollbar-width thin
    scrollbar-color #555 #2d2d30

    &::-webkit-scrollbar
        height 4px

    &::-webkit-scrollbar-track
        background #2d2d30

    &::-webkit-scrollbar-thumb
        background #555
        border-radius 2px

.tab
    display flex
    align-items center
    gap 6px
    padding 6px 12px
    background #2d2d30
    color #999
    cursor pointer
    border-right 1px solid #3e3e42
    white-space nowrap
    transition all 0.15s ease

    &:hover
        background #383838
        color #ccc

    &.active
        background #1e1e1e
        color #fff
        border-bottom 2px solid #0078d4

.tab-title
    font-size 12px
    font-family 'JetBrains Mono', 'Consolas', monospace
    max-width 120px
    overflow hidden
    text-overflow ellipsis

.tab-close
    display flex
    align-items center
    justify-content center
    width 16px
    height 16px
    padding 0
    background none
    border none
    color #666
    cursor pointer
    border-radius 3px
    transition all 0.15s ease

    &:hover
        background rgba(255, 255, 255, 0.1)
        color #fff

    .icon
        width 10px
        height 10px

.tabs-actions
    display flex
    align-items center
    gap 4px
    padding 0 8px

.btn-add-tab
.btn-popout
    display flex
    align-items center
    justify-content center
    width 28px
    height 28px
    padding 0
    background none
    border none
    color #999
    cursor pointer
    border-radius 4px
    transition all 0.15s ease

    &:hover
        background rgba(255, 255, 255, 0.1)
        color #fff

    .icon
        width 16px
        height 16px

.tabs-content
    flex 1
    position relative
    overflow hidden

.tab-panel
    position absolute
    top 0
    left 0
    right 0
    bottom 0
    display none

    &.active
        display block
</style>
