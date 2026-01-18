<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue';
import { GridLayout, GridItem } from 'vue-grid-layout';
import TerminalTabs from './TerminalTabs.vue';

interface TerminalTab {
    id: string;
    title: string;
    sessionId: string | null;
    workingDirectory: string;
}

interface GridItemData {
    i: string;
    x: number;
    y: number;
    w: number;
    h: number;
    tabs: TerminalTab[];
}

const props = defineProps<{
    workingDirectory?: string;
}>();

const emit = defineEmits<{
    (e: 'layout-change', layout: GridItemData[]): void;
    (e: 'popout', tab: TerminalTab, gridItemId: string): void;
}>();

const layout = ref<GridItemData[]>([]);
const colNum = ref(12);
const rowHeight = ref(50);
const terminalTabsRefs = ref<Record<string, InstanceType<typeof TerminalTabs>>>({});

function generateId(): string {
    return `grid-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

function addGridItem() {
    // Find first available position
    let y = 0;

    if (layout.value.length > 0) {
        // Find the max y position and add below
        const maxY = Math.max(...layout.value.map(item => item.y + item.h));
        y = maxY;
    }

    const newItem: GridItemData = {
        i: generateId(),
        x: 0,
        y,
        w: 6,
        h: 6,
        tabs: [],
    };

    layout.value.push(newItem);
    saveLayout();
}

function removeGridItem(itemId: string) {
    const index = layout.value.findIndex(item => item.i === itemId);
    if (index !== -1) {
        layout.value.splice(index, 1);
        saveLayout();
    }
}

function onTabsChange(itemId: string, tabs: TerminalTab[]) {
    const item = layout.value.find(i => i.i === itemId);
    if (item) {
        item.tabs = tabs;
        saveLayout();
    }
}

function onPopout(tab: TerminalTab, itemId: string) {
    emit('popout', tab, itemId);
}

function onLayoutUpdated(newLayout: any[]) {
    // Update layout positions
    newLayout.forEach(item => {
        const gridItem = layout.value.find(i => i.i === item.i);
        if (gridItem) {
            gridItem.x = item.x;
            gridItem.y = item.y;
            gridItem.w = item.w;
            gridItem.h = item.h;
        }
    });
    saveLayout();

    // Trigger resize on all terminals (handled by ResizeObserver in XTerminal)
    nextTick(() => {
        // ResizeObserver in XTerminal handles terminal resize automatically
    });
}

function saveLayout() {
    const toSave = layout.value.map(item => ({
        i: item.i,
        x: item.x,
        y: item.y,
        w: item.w,
        h: item.h,
        tabs: item.tabs,
    }));
    localStorage.setItem('terminal_grid_layout', JSON.stringify(toSave));
    emit('layout-change', layout.value);
}

function loadLayout() {
    const saved = localStorage.getItem('terminal_grid_layout');
    if (saved) {
        try {
            const parsed = JSON.parse(saved);
            // Reset session IDs on load (sessions are not persistent)
            layout.value = parsed.map((item: GridItemData) => ({
                ...item,
                tabs: item.tabs.map(tab => ({
                    ...tab,
                    sessionId: null,
                })),
            }));
        } catch (e) {
            console.error('Failed to parse saved layout:', e);
            layout.value = [];
        }
    }

    // Add default item if empty
    if (layout.value.length === 0) {
        addGridItem();
    }
}

function setTerminalTabsRef(itemId: string, el: any) {
    if (el) {
        terminalTabsRefs.value[itemId] = el;
    } else {
        delete terminalTabsRefs.value[itemId];
    }
}

onMounted(() => {
    loadLayout();
});

// Expose methods
defineExpose({
    addGridItem,
    removeGridItem,
    getLayout: () => layout.value,
});
</script>

<template>
    <div class="terminal-grid">
        <GridLayout
            v-model:layout="layout"
            :col-num="colNum"
            :row-height="rowHeight"
            :is-draggable="true"
            :is-resizable="true"
            :vertical-compact="true"
            :use-css-transforms="true"
            :margin="[8, 8]"
            @layout-updated="onLayoutUpdated"
        >
            <GridItem
                v-for="item in layout"
                :key="item.i"
                :i="item.i"
                :x="item.x"
                :y="item.y"
                :w="item.w"
                :h="item.h"
                :min-w="3"
                :min-h="4"
                drag-allow-from=".grid-item-header"
                drag-ignore-from=".tabs-header, .xterm-container"
            >
                <div class="grid-item-wrapper">
                    <div class="grid-item-header">
                        <span class="grip-handle">
                            <svg viewBox="0 0 24 24" class="icon">
                                <circle cx="5" cy="5" r="1.5" fill="currentColor"/>
                                <circle cx="12" cy="5" r="1.5" fill="currentColor"/>
                                <circle cx="19" cy="5" r="1.5" fill="currentColor"/>
                                <circle cx="5" cy="12" r="1.5" fill="currentColor"/>
                                <circle cx="12" cy="12" r="1.5" fill="currentColor"/>
                                <circle cx="19" cy="12" r="1.5" fill="currentColor"/>
                            </svg>
                        </span>
                        <button
                            class="btn-close-grid"
                            @click="removeGridItem(item.i)"
                            title="Close panel"
                        >
                            <svg viewBox="0 0 24 24" class="icon">
                                <line x1="18" y1="6" x2="6" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                <line x1="6" y1="6" x2="18" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            </svg>
                        </button>
                    </div>
                    <div class="grid-item-content">
                        <TerminalTabs
                            :ref="(el) => setTerminalTabsRef(item.i, el)"
                            :working-directory="props.workingDirectory"
                            :initial-tabs="item.tabs"
                            @tabs-change="(tabs) => onTabsChange(item.i, tabs)"
                            @popout="(tab) => onPopout(tab, item.i)"
                        />
                    </div>
                </div>
            </GridItem>
        </GridLayout>
    </div>
</template>

<style scoped lang="stylus">
.terminal-grid
    width 100%
    height 100%
    overflow auto

:deep(.vue-grid-layout)
    min-height 100%

:deep(.vue-grid-item)
    transition transform 0.2s ease

    &.vue-grid-placeholder
        background #0078d4 !important
        opacity 0.2

    &.vue-draggable-dragging
        transition none
        z-index 100

:deep(.vue-resizable-handle)
    position absolute
    width 20px
    height 20px
    bottom 0
    right 0
    background url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='6' height='6' fill='%23666'%3E%3Ccircle cx='3' cy='3' r='1.5'/%3E%3C/svg%3E") no-repeat center
    cursor se-resize

.grid-item-wrapper
    display flex
    flex-direction column
    height 100%
    background #1e1e1e
    border-radius 6px
    overflow hidden
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.3)

.grid-item-header
    display flex
    align-items center
    justify-content space-between
    padding 4px 8px
    background #252526
    border-bottom 1px solid #3e3e42
    cursor grab

    &:active
        cursor grabbing

.grip-handle
    display flex
    align-items center
    color #666

    .icon
        width 16px
        height 16px

.btn-close-grid
    display flex
    align-items center
    justify-content center
    width 20px
    height 20px
    padding 0
    background none
    border none
    color #666
    cursor pointer
    border-radius 3px
    transition all 0.15s ease

    &:hover
        background rgba(255, 255, 255, 0.1)
        color #f14c4c

    .icon
        width 12px
        height 12px

.grid-item-content
    flex 1
    overflow hidden
</style>
