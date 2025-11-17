<script setup lang="ts">
import { ref } from 'vue';

const content = ref('');
const fileName = ref('Untitled.txt');

function saveFile() {
    // TODO: Реализовать сохранение файла через Tauri
    alert('Функция сохранения будет реализована позже');
}

function openFile() {
    // TODO: Реализовать открытие файла через Tauri
    alert('Функция открытия файла будет реализована позже');
}

function clearEditor() {
    if (confirm('Вы уверены, что хотите очистить редактор?')) {
        content.value = '';
        fileName.value = 'Untitled.txt';
    }
}
</script>

<template>
    <div class="page">
        <div class="editor-header">
            <h1 class="page-title">Editor</h1>
            <div class="file-name">{{ fileName }}</div>
        </div>

        <div class="toolbar">
            <button @click="openFile" class="toolbar-btn">
                <svg viewBox="0 0 24 24" class="btn-icon">
                    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <polyline points="17 21 17 13 7 13 7 21" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <polyline points="7 3 7 8 15 8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                Открыть
            </button>
            <button @click="saveFile" class="toolbar-btn toolbar-btn-primary">
                <svg viewBox="0 0 24 24" class="btn-icon">
                    <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <polyline points="17 21 17 13 7 13 7 21" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <polyline points="7 3 7 8 15 8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                Сохранить
            </button>
            <button @click="clearEditor" class="toolbar-btn">
                <svg viewBox="0 0 24 24" class="btn-icon">
                    <polyline points="3 6 5 6 21 6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                Очистить
            </button>
        </div>

        <div class="editor-container">
            <textarea
                v-model="content"
                class="editor-textarea"
                placeholder="Начните вводить текст..."
                spellcheck="false"
            ></textarea>
        </div>

        <div class="editor-footer">
            <div class="stats">
                <span class="stat-item">Символов: {{ content.length }}</span>
                <span class="stat-item">Строк: {{ content.split('\n').length }}</span>
                <span class="stat-item">Слов: {{ content.trim() ? content.trim().split(/\s+/).length : 0 }}</span>
            </div>
        </div>
    </div>
</template>

<style scoped lang="stylus">
.page
    display flex
    flex-direction column
    height calc(100vh - 40px)
    padding 20px
    max-width 1400px
    margin 0 auto

.editor-header
    display flex
    align-items center
    justify-content space-between
    margin-bottom 20px

.page-title
    font-size 36px
    font-weight 700
    color #0052cc
    margin 0

.file-name
    font-size 16px
    color #6B778C
    font-weight 500
    padding 8px 16px
    background-color #F4F5F7
    border-radius 6px

.toolbar
    display flex
    gap 12px
    margin-bottom 20px
    padding 16px
    background white
    border-radius 8px
    box-shadow 0 2px 4px rgba(0, 0, 0, 0.1)

.toolbar-btn
    display flex
    align-items center
    gap 8px
    padding 10px 20px
    background white
    border 2px solid #DFE1E6
    border-radius 6px
    font-size 14px
    font-weight 600
    color #172B4D
    cursor pointer
    transition all 0.2s ease

    &:hover
        background-color #F4F5F7
        border-color #C1C7D0

    &:active
        transform scale(0.98)

.toolbar-btn-primary
    background-color #0052cc
    border-color #0052cc
    color white

    &:hover
        background-color #0747a6
        border-color #0747a6

.btn-icon
    width 18px
    height 18px

.editor-container
    flex 1
    background white
    border-radius 8px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)
    overflow hidden
    margin-bottom 20px

.editor-textarea
    width 100%
    height 100%
    padding 24px
    border none
    outline none
    font-family 'JetBrains Mono', 'Fira Code', 'Courier New', monospace
    font-size 14px
    line-height 1.6
    color #172B4D
    resize none
    background white

    &::placeholder
        color #C1C7D0

.editor-footer
    padding 16px
    background white
    border-radius 8px
    box-shadow 0 2px 4px rgba(0, 0, 0, 0.1)

.stats
    display flex
    gap 24px
    justify-content flex-start

.stat-item
    font-size 14px
    color #6B778C
    font-weight 500

    &:before
        content '•'
        margin-right 8px
        color #C1C7D0

    &:first-child:before
        content ''
        margin-right 0
</style>
