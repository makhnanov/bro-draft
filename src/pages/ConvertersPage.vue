<script setup lang="ts">
import { ref } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';

const selectedFile = ref<string | null>(null);
const fileExtension = ref<string>('');
const isConverting = ref(false);
const conversionStatus = ref('');
const outputPath = ref<string | null>(null);
const copySuccess = ref(false);

async function selectFile() {
    const file = await open({
        multiple: false,
        directory: false,
        filters: [{
            name: 'Video Files',
            extensions: ['flv', 'mp4', 'avi', 'mkv', 'mov', 'wmv']
        }]
    });

    if (file) {
        selectedFile.value = file;
        const parts = file.split('.');
        fileExtension.value = parts[parts.length - 1].toLowerCase();
        conversionStatus.value = '';
        outputPath.value = null;
        copySuccess.value = false;
    }
}

async function convertToMP4() {
    if (!selectedFile.value) return;

    isConverting.value = true;
    conversionStatus.value = 'Конвертация...';
    outputPath.value = null;
    copySuccess.value = false;

    try {
        const result = await invoke<string>('convert_to_mp4', {
            inputPath: selectedFile.value
        });

        outputPath.value = result;
        conversionStatus.value = `Готово! Файл сохранен: ${result}`;
    } catch (error) {
        console.error('Conversion error:', error);
        conversionStatus.value = `Ошибка: ${error}`;
        outputPath.value = null;
    } finally {
        isConverting.value = false;
    }
}

async function copyToClipboard() {
    if (!outputPath.value) return;

    try {
        await navigator.clipboard.writeText(outputPath.value);
        copySuccess.value = true;
        setTimeout(() => {
            copySuccess.value = false;
        }, 2000);
    } catch (error) {
        console.error('Failed to copy to clipboard:', error);
    }
}

function resetSelection() {
    selectedFile.value = null;
    fileExtension.value = '';
    conversionStatus.value = '';
    outputPath.value = null;
    copySuccess.value = false;
}
</script>

<template>
    <div class="page">
        <h1 class="page-title">Video Converter</h1>

        <div class="converter-container">
            <div class="upload-section">
                <button @click="selectFile" class="btn-select" :disabled="isConverting">
                    <svg viewBox="0 0 24 24" class="btn-icon">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                    Выбрать файл
                </button>

                <div v-if="selectedFile" class="file-info">
                    <div class="file-header">
                        <svg viewBox="0 0 24 24" class="file-icon">
                            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                            <polyline points="14 2 14 8 20 8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                        <div class="file-details">
                            <div class="file-name">{{ selectedFile.split('/').pop()?.split('\\').pop() }}</div>
                            <div class="file-ext">Расширение: {{ fileExtension.toUpperCase() }}</div>
                        </div>
                        <button @click="resetSelection" class="btn-close" :disabled="isConverting">
                            <svg viewBox="0 0 24 24" class="icon">
                                <line x1="18" y1="6" x2="6" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                <line x1="6" y1="6" x2="18" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            </svg>
                        </button>
                    </div>

                    <div v-if="fileExtension === 'flv' || fileExtension === 'mkv'" class="conversion-section">
                        <div class="conversion-info">
                            <svg viewBox="0 0 24 24" class="info-icon">
                                <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/>
                                <line x1="12" y1="16" x2="12" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                <line x1="12" y1="8" x2="12.01" y2="8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            </svg>
                            <p>Обнаружен {{ fileExtension.toUpperCase() }} файл. Конвертировать в MP4?</p>
                        </div>

                        <button
                            @click="convertToMP4"
                            class="btn-convert"
                            :disabled="isConverting"
                        >
                            <svg viewBox="0 0 24 24" class="btn-icon">
                                <polyline points="16 3 21 3 21 8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                                <line x1="4" y1="20" x2="21" y2="3" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                                <polyline points="21 16 21 21 16 21" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                                <line x1="15" y1="15" x2="21" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                            </svg>
                            {{ isConverting ? 'Конвертация...' : 'Конвертировать в MP4' }}
                        </button>
                    </div>

                    <div v-else class="no-conversion">
                        <p>Конвертация для файлов .{{ fileExtension }} пока не поддерживается</p>
                    </div>

                    <div v-if="conversionStatus" :class="['status-message', { error: conversionStatus.includes('Ошибка') }]">
                        <div class="status-content">
                            <span>{{ conversionStatus }}</span>
                            <button
                                v-if="outputPath && !conversionStatus.includes('Ошибка')"
                                @click="copyToClipboard"
                                class="btn-copy"
                                :class="{ copied: copySuccess }"
                            >
                                <svg v-if="!copySuccess" viewBox="0 0 24 24" class="copy-icon">
                                    <rect x="9" y="9" width="13" height="13" rx="2" ry="2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                                    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                                </svg>
                                <svg v-else viewBox="0 0 24 24" class="copy-icon">
                                    <polyline points="20 6 9 17 4 12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                                </svg>
                                {{ copySuccess ? 'Скопировано!' : 'Копировать путь' }}
                            </button>
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
    padding 40px
    max-width 800px
    margin 0 auto

.page-title
    font-size 36px
    font-weight 700
    color #0052cc
    margin-bottom 40px
    text-align center

.converter-container
    background white
    border-radius 12px
    padding 40px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)

.upload-section
    display flex
    flex-direction column
    align-items center
    gap 30px

.btn-select
    display flex
    align-items center
    gap 10px
    padding 16px 32px
    background-color #0052cc
    color white
    border none
    border-radius 8px
    font-size 16px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover:not(:disabled)
        background-color #0747a6
        transform translateY(-2px)
        box-shadow 0 4px 12px rgba(0, 82, 204, 0.3)

    &:disabled
        background-color #C1C7D0
        cursor not-allowed

.btn-icon
    width 20px
    height 20px

.file-info
    width 100%
    display flex
    flex-direction column
    gap 20px

.file-header
    display flex
    align-items center
    gap 16px
    padding 20px
    background-color #F4F5F7
    border-radius 8px

.file-icon
    width 40px
    height 40px
    color #0052cc
    flex-shrink 0

.file-details
    flex 1

.file-name
    font-size 16px
    font-weight 600
    color #172B4D
    margin-bottom 4px
    word-break break-all

.file-ext
    font-size 13px
    color #6B778C

.btn-close
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

    &:hover:not(:disabled)
        background-color rgba(222, 53, 11, 0.1)

    &:disabled
        opacity 0.5
        cursor not-allowed

.icon
    width 20px
    height 20px

.conversion-section
    display flex
    flex-direction column
    gap 16px

.conversion-info
    display flex
    align-items center
    gap 12px
    padding 16px
    background-color #E3FCEF
    border-left 4px solid #36B37E
    border-radius 4px

    p
        margin 0
        font-size 14px
        color #172B4D

.info-icon
    width 24px
    height 24px
    color #36B37E
    flex-shrink 0

.btn-convert
    display flex
    align-items center
    justify-content center
    gap 10px
    padding 14px 28px
    background-color #36B37E
    color white
    border none
    border-radius 8px
    font-size 15px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover:not(:disabled)
        background-color #2a9d68
        transform translateY(-2px)
        box-shadow 0 4px 12px rgba(54, 179, 126, 0.3)

    &:disabled
        background-color #C1C7D0
        cursor not-allowed

.no-conversion
    padding 16px
    background-color #FFF0B3
    border-left 4px solid #FFAB00
    border-radius 4px
    text-align center

    p
        margin 0
        font-size 14px
        color #172B4D

.status-message
    padding 16px
    background-color #E3FCEF
    border-left 4px solid #36B37E
    border-radius 4px
    font-size 14px
    color #172B4D

    &.error
        background-color #FFEBE6
        border-left-color #DE350B
        color #DE350B

.status-content
    display flex
    align-items center
    justify-content space-between
    gap 16px
    flex-wrap wrap

.btn-copy
    display flex
    align-items center
    gap 6px
    padding 8px 16px
    background-color #0052cc
    color white
    border none
    border-radius 6px
    font-size 13px
    font-weight 600
    cursor pointer
    transition all 0.2s ease
    white-space nowrap
    flex-shrink 0

    &:hover
        background-color #0747a6
        transform translateY(-1px)

    &.copied
        background-color #36B37E

        &:hover
            background-color #2a9d68

.copy-icon
    width 16px
    height 16px
    flex-shrink 0
</style>
