<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface SSHServer {
    id: number;
    name: string;
    command: string;
    showCommand: boolean;
}

const servers = ref<SSHServer[]>([]);

onMounted(() => {
    loadServers();
});

function loadServers() {
    const saved = localStorage.getItem('ssh_servers');
    if (saved) {
        servers.value = JSON.parse(saved).map((s: SSHServer) => ({
            ...s,
            showCommand: false
        }));
    }
}

function saveServers() {
    localStorage.setItem('ssh_servers', JSON.stringify(servers.value));
}

function addServer() {
    const newServer: SSHServer = {
        id: Date.now(),
        name: 'New Executor',
        command: '',
        showCommand: false
    };

    servers.value.push(newServer);
    saveServers();
}

function deleteServer(id: number) {
    servers.value = servers.value.filter(s => s.id !== id);
    saveServers();
}

function updateServerName(id: number, name: string) {
    const server = servers.value.find(s => s.id === id);
    if (server) {
        server.name = name;
        saveServers();
    }
}

function updateServerCommand(id: number, command: string) {
    const server = servers.value.find(s => s.id === id);
    if (server) {
        server.command = command;
        saveServers();
    }
}

function toggleCommandVisibility(id: number) {
    const server = servers.value.find(s => s.id === id);
    if (server) {
        server.showCommand = !server.showCommand;
    }
}

async function connectToServer(command: string) {
    try {
        await invoke('open_terminal', { command });
    } catch (error) {
        console.error('Failed to open terminal:', error);
        alert('Ошибка при открытии терминала: ' + error);
    }
}
</script>

<template>
    <div class="page">
        <div class="page-header">
            <h1 class="page-title">Commands</h1>
            <button @click="addServer" class="btn-add">
                <svg viewBox="0 0 24 24" class="btn-icon">
                    <line x1="12" y1="5" x2="12" y2="19" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    <line x1="5" y1="12" x2="19" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                </svg>
                Add
            </button>
        </div>

        <div v-if="servers.length === 0" class="empty-state">
            <p>No commands. Click "Add" to create one.</p>
        </div>

        <div v-else class="servers-table">
            <div v-for="server in servers" :key="server.id" class="server-row">
                <button @click="deleteServer(server.id)" class="btn-delete" title="Удалить">
                    <svg viewBox="0 0 24 24" class="icon">
                        <line x1="18" y1="6" x2="6" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                        <line x1="6" y1="6" x2="18" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    </svg>
                </button>

                <input
                    :value="server.name"
                    @input="updateServerName(server.id, ($event.target as HTMLInputElement).value)"
                    class="input-name"
                    placeholder="Executor Name"
                />

                <input
                    :value="server.command"
                    :type="server.showCommand ? 'text' : 'password'"
                    @input="updateServerCommand(server.id, ($event.target as HTMLInputElement).value)"
                    class="input-command"
                    placeholder="command to execute"
                />

                <button
                    @click="toggleCommandVisibility(server.id)"
                    class="btn-toggle"
                    :title="server.showCommand ? 'Скрыть' : 'Показать'"
                >
                    <svg v-if="!server.showCommand" viewBox="0 0 24 24" class="icon">
                        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" fill="none" stroke="currentColor" stroke-width="2"/>
                        <circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" stroke-width="2"/>
                    </svg>
                    <svg v-else viewBox="0 0 24 24" class="icon">
                        <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                        <line x1="1" y1="1" x2="23" y2="23" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    </svg>
                </button>

                <button @click="connectToServer(server.command)" class="btn-connect">
                    Подключить
                </button>
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

.servers-table
    display flex
    flex-direction column
    gap 4px

.server-row
    display flex
    align-items center
    gap 8px
    padding 8px
    background white
    border-radius 6px
    box-shadow 0 1px 3px rgba(0, 0, 0, 0.1)
    transition all 0.2s ease

    &:hover
        box-shadow 0 2px 6px rgba(0, 0, 0, 0.15)

.btn-delete
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

.icon
    width 18px
    height 18px

.input-name
    flex-shrink 0
    width 180px
    padding 6px 10px
    border 1px solid #DFE1E6
    border-radius 4px
    font-size 14px
    font-weight 500
    color #172B4D
    transition border-color 0.2s ease

    &:focus
        outline none
        border-color #0052cc

.input-command
    flex 1
    padding 6px 10px
    border 1px solid #DFE1E6
    border-radius 4px
    font-size 13px
    font-family 'JetBrains Mono', 'Courier New', monospace
    color #172B4D
    transition border-color 0.2s ease

    &:focus
        outline none
        border-color #0052cc

    &[type="password"]
        font-family inherit
        letter-spacing 3px

.btn-toggle
    flex-shrink 0
    width 32px
    height 32px
    display flex
    align-items center
    justify-content center
    background none
    border 1px solid #DFE1E6
    border-radius 4px
    cursor pointer
    color #6B778C
    transition all 0.2s ease

    &:hover
        background-color #F4F5F7
        border-color #C1C7D0

.btn-connect
    flex-shrink 0
    padding 6px 16px
    background-color #0052cc
    color white
    border none
    border-radius 4px
    font-size 13px
    font-weight 600
    cursor pointer
    transition all 0.2s ease
    white-space nowrap

    &:hover
        background-color #0747a6
</style>
