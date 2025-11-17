<script setup lang="ts">
import { ref, onMounted } from 'vue';

const appVersion = ref('1.0.0');
const theme = ref('auto');
const language = ref('en');
const notifications = ref(true);

onMounted(() => {
    // Загрузка настроек из localStorage
    const savedTheme = localStorage.getItem('theme');
    const savedLanguage = localStorage.getItem('language');
    const savedNotifications = localStorage.getItem('notifications');

    if (savedTheme) theme.value = savedTheme;
    if (savedLanguage) language.value = savedLanguage;
    if (savedNotifications) notifications.value = savedNotifications === 'true';
});

function saveSettings() {
    localStorage.setItem('theme', theme.value);
    localStorage.setItem('language', language.value);
    localStorage.setItem('notifications', notifications.value.toString());
    alert('Настройки сохранены');
}

function resetSettings() {
    if (confirm('Вы уверены, что хотите сбросить все настройки?')) {
        theme.value = 'auto';
        language.value = 'en';
        notifications.value = true;
        localStorage.removeItem('theme');
        localStorage.removeItem('language');
        localStorage.removeItem('notifications');
        alert('Настройки сброшены');
    }
}
</script>

<template>
    <div class="page">
        <h1 class="page-title">Settings</h1>

        <div class="settings-container">
            <!-- Внешний вид -->
            <div class="section">
                <h2 class="section-title">Внешний вид</h2>

                <div class="setting-item">
                    <label for="theme" class="setting-label">Тема</label>
                    <select v-model="theme" id="theme" class="setting-select">
                        <option value="auto">Автоматически</option>
                        <option value="light">Светлая</option>
                        <option value="dark">Тёмная</option>
                    </select>
                </div>
            </div>

            <!-- Язык -->
            <div class="section">
                <h2 class="section-title">Язык и регион</h2>

                <div class="setting-item">
                    <label for="language" class="setting-label">Язык интерфейса</label>
                    <select v-model="language" id="language" class="setting-select">
                        <option value="en">English</option>
                        <option value="ru">Русский</option>
                        <option value="kk">Қазақша</option>
                    </select>
                </div>
            </div>

            <!-- Уведомления -->
            <div class="section">
                <h2 class="section-title">Уведомления</h2>

                <div class="setting-item">
                    <label class="setting-label">
                        <input type="checkbox" v-model="notifications" class="setting-checkbox">
                        <span>Включить уведомления</span>
                    </label>
                </div>
            </div>

            <!-- О программе -->
            <div class="section">
                <h2 class="section-title">О программе</h2>

                <div class="info-item">
                    <span class="info-label">Версия:</span>
                    <span class="info-value">{{ appVersion }}</span>
                </div>
                <div class="info-item">
                    <span class="info-label">Разработчик:</span>
                    <span class="info-value">BroLauncher Team</span>
                </div>
            </div>

            <!-- Кнопки действий -->
            <div class="actions">
                <button @click="saveSettings" class="btn btn-primary">
                    Сохранить настройки
                </button>
                <button @click="resetSettings" class="btn btn-secondary">
                    Сбросить настройки
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped lang="stylus">
.page
    display flex
    flex-direction column
    padding 40px
    max-width 900px
    margin 0 auto

.page-title
    font-size 36px
    font-weight 700
    color #0052cc
    margin-bottom 40px

.settings-container
    display flex
    flex-direction column
    gap 20px

.section
    background white
    border-radius 12px
    padding 30px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)

.section-title
    font-size 20px
    font-weight 600
    color #172B4D
    margin-bottom 20px
    padding-bottom 10px
    border-bottom 2px solid #DFE1E6

.setting-item
    margin-bottom 20px

    &:last-child
        margin-bottom 0

.setting-label
    display block
    font-size 14px
    font-weight 500
    color #172B4D
    margin-bottom 8px

.setting-select
    width 100%
    padding 10px 16px
    border 2px solid #DFE1E6
    border-radius 8px
    font-size 14px
    font-family inherit
    background-color white
    cursor pointer
    transition border-color 0.2s ease

    &:focus
        outline none
        border-color #0052cc

.setting-checkbox
    margin-right 10px
    width 18px
    height 18px
    cursor pointer

.info-item
    display flex
    justify-content space-between
    padding 12px 0
    border-bottom 1px solid #F4F5F7

    &:last-child
        border-bottom none

.info-label
    font-size 14px
    font-weight 500
    color #6B778C

.info-value
    font-size 14px
    color #172B4D
    font-weight 500

.actions
    display flex
    gap 16px
    margin-top 20px

.btn
    padding 12px 32px
    border none
    border-radius 8px
    font-size 16px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover
        transform translateY(-2px)
        box-shadow 0 4px 12px rgba(0, 82, 204, 0.3)

    &:active
        transform translateY(0)

.btn-primary
    background-color #0052cc
    color white

    &:hover
        background-color #0747a6

.btn-secondary
    background-color #DFE1E6
    color #172B4D

    &:hover
        background-color #C1C7D0
</style>
