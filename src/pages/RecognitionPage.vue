<script setup lang="ts">
import { ref } from 'vue';

const activeTab = ref('voice');

function setActiveTab(tab: string) {
    activeTab.value = tab;
}
</script>

<template>
    <div class="page">
        <h1 class="page-title">Recognition</h1>

        <!-- Табы -->
        <div class="tabs">
            <button
                :class="['tab', { active: activeTab === 'voice' }]"
                @click="setActiveTab('voice')"
            >
                <svg viewBox="0 0 24 24" class="tab-icon">
                    <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M19 11c0 3.87-3.13 7-7 7s-7-3.13-7-7M12 18v4m-4 0h8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                Voice
            </button>
            <button
                :class="['tab', { active: activeTab === 'screen' }]"
                @click="setActiveTab('screen')"
            >
                <svg viewBox="0 0 24 24" class="tab-icon">
                    <rect x="2" y="3" width="20" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M8 21h8M12 17v4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                Screen
            </button>
        </div>

        <!-- Контент Voice -->
        <div v-show="activeTab === 'voice'" class="tab-content">
            <div class="section">
                <h2 class="section-title">Голосовое распознавание</h2>
                <p class="section-description">Распознавание и обработка голосовых команд</p>

                <div class="content-placeholder">
                    <svg viewBox="0 0 24 24" class="placeholder-icon">
                        <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                        <path d="M19 11c0 3.87-3.13 7-7 7s-7-3.13-7-7M12 18v4m-4 0h8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                    <p class="placeholder-text">Функционал голосового распознавания будет добавлен позже</p>
                </div>
            </div>
        </div>

        <!-- Контент Screen -->
        <div v-show="activeTab === 'screen'" class="tab-content">
            <div class="section">
                <h2 class="section-title">Распознавание экрана</h2>
                <p class="section-description">OCR и распознавание текста на экране</p>

                <div class="content-placeholder">
                    <svg viewBox="0 0 24 24" class="placeholder-icon">
                        <rect x="2" y="3" width="20" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                        <path d="M8 21h8M12 17v4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                    <p class="placeholder-text">Функционал распознавания экрана будет добавлен позже</p>
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
    max-width 1200px
    margin 0 auto

.page-title
    font-size 36px
    font-weight 700
    color #0052cc
    margin-bottom 30px

.tabs
    display flex
    gap 8px
    margin-bottom 30px
    border-bottom 2px solid #DFE1E6

.tab
    display flex
    align-items center
    gap 8px
    padding 12px 24px
    background none
    border none
    border-bottom 3px solid transparent
    font-size 16px
    font-weight 600
    color #6B778C
    cursor pointer
    transition all 0.2s ease
    margin-bottom -2px

    &:hover
        color #0052cc
        background-color rgba(0, 82, 204, 0.05)

    &.active
        color #0052cc
        border-bottom-color #0052cc

.tab-icon
    width 20px
    height 20px

.tab-content
    animation fadeIn 0.3s ease

@keyframes fadeIn
    from
        opacity 0
        transform translateY(10px)
    to
        opacity 1
        transform translateY(0)

.section
    background white
    border-radius 12px
    padding 30px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)

.section-title
    font-size 24px
    font-weight 600
    color #172B4D
    margin-bottom 10px

.section-description
    font-size 14px
    color #6B778C
    margin-bottom 30px
    line-height 1.6

.content-placeholder
    display flex
    flex-direction column
    align-items center
    justify-content center
    padding 60px 20px
    text-align center

.placeholder-icon
    width 80px
    height 80px
    color #C1C7D0
    margin-bottom 20px

.placeholder-text
    font-size 16px
    color #6B778C
    max-width 400px
</style>
