<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

// Типы действий
type ActionType = 'click' | 'keypress';

interface ClickAction {
  type: 'click';
  x: number;
  y: number;
  button: 'left' | 'right';
}

interface KeypressAction {
  type: 'keypress';
  keys: string; // например "Ctrl+C", "Alt+Tab"
}

type Action = ClickAction | KeypressAction;

interface ButtonTemplate {
  id: string;
  name: string;
  actions: Action[];
  isActive: boolean;
}

const templates = ref<ButtonTemplate[]>([]);
const editingTemplate = ref<ButtonTemplate | null>(null);
const isCreating = ref(false);
const newTemplateName = ref('');
const actionType = ref<ActionType>('click');

// Для записи кликов
const isRecordingClicks = ref(false);
const recordedClicks = ref<ClickAction[]>([]);

// Для ввода горячих клавиш
const hotkey = ref('');
const isEditingName = ref<string | null>(null);
const editingName = ref('');
const isCapturingKeys = ref(false); // Режим перехвата клавиш

// Загрузка шаблонов из файловой системы
onMounted(async () => {
  try {
    const saved = await invoke<string>('load_button_templates');
    if (saved) {
      templates.value = JSON.parse(saved);
    }
  } catch (error) {
    console.error('Failed to load templates:', error);
  }

  // Слушаем событие закрытия оверлейной кнопки
  window.addEventListener('overlay-button-closed', handleOverlayClosed);
});

onUnmounted(() => {
  window.removeEventListener('overlay-button-closed', handleOverlayClosed);
});

// Обработчик закрытия оверлея
function handleOverlayClosed() {
  // Деактивируем все кнопки
  templates.value.forEach(t => t.isActive = false);
  saveTemplates();
}

// Сохранение шаблонов
async function saveTemplates() {
  try {
    await invoke('save_button_templates', { templatesJson: JSON.stringify(templates.value) });
  } catch (error) {
    console.error('Failed to save templates:', error);
    alert('Ошибка при сохранении шаблонов: ' + error);
  }
}

// Создание нового шаблона
function createNewTemplate() {
  if (!newTemplateName.value.trim()) {
    alert('Введите название кнопки');
    return;
  }

  const template: ButtonTemplate = {
    id: Date.now().toString(),
    name: newTemplateName.value,
    actions: [],
    isActive: false
  };

  templates.value.push(template);
  editingTemplate.value = template;
  isCreating.value = false;
  newTemplateName.value = '';
  saveTemplates();
}

// Удаление шаблона
function deleteTemplate(id: string) {
  if (confirm('Удалить этот шаблон?')) {
    templates.value = templates.value.filter(t => t.id !== id);
    if (editingTemplate.value?.id === id) {
      editingTemplate.value = null;
    }
    saveTemplates();
  }
}

// Начать редактирование шаблона
function editTemplate(template: ButtonTemplate) {
  editingTemplate.value = template;
}

// Начать редактирование названия
function startEditingName(template: ButtonTemplate) {
  isEditingName.value = template.id;
  editingName.value = template.name;
}

// Сохранить новое название
function saveTemplateName(template: ButtonTemplate) {
  if (editingName.value.trim()) {
    template.name = editingName.value.trim();
    saveTemplates();
  }
  isEditingName.value = null;
  editingName.value = '';
}

// Отменить редактирование названия
function cancelEditingName() {
  isEditingName.value = null;
  editingName.value = '';
}

// Добавить действие клика
function addClickAction() {
  if (!editingTemplate.value) return;

  const action: ClickAction = {
    type: 'click',
    x: 100,
    y: 100,
    button: 'left'
  };

  editingTemplate.value.actions.push(action);
  saveTemplates();
}

// Начать запись кликов
async function startRecordingClicks() {
  try {
    recordedClicks.value = [];
    isRecordingClicks.value = true;
    await invoke('start_click_recording');
    console.log('Click recording started');
  } catch (error) {
    console.error('Error starting recording:', error);
    alert('Ошибка при запуске записи: ' + error);
    isRecordingClicks.value = false;
  }
}

// Остановить запись кликов
async function stopRecordingClicks() {
  try {
    const sequence = await invoke<Array<{x: number, y: number, monitor: number, button: string}>>('stop_click_recording');
    // Удаляем последний клик (это клик по кнопке "Завершить запись")
    if (sequence.length > 0) {
      sequence.pop();
    }

    // Преобразуем в наш формат
    const clicks: ClickAction[] = sequence.map(click => ({
      type: 'click',
      x: click.x,
      y: click.y,
      button: click.button as 'left' | 'right'
    }));

    if (editingTemplate.value) {
      editingTemplate.value.actions.push(...clicks);
      saveTemplates();
    }

    isRecordingClicks.value = false;
    console.log('Recording stopped, clicks:', clicks.length);
  } catch (error) {
    console.error('Error stopping recording:', error);
    alert('Ошибка при остановке записи: ' + error);
    isRecordingClicks.value = false;
  }
}

// Добавить действие горячих клавиш
function addKeypressAction() {
  if (!editingTemplate.value || !hotkey.value.trim()) {
    alert('Введите сочетание клавиш');
    return;
  }

  const action: KeypressAction = {
    type: 'keypress',
    keys: hotkey.value
  };

  editingTemplate.value.actions.push(action);
  hotkey.value = '';
  saveTemplates();
}

// Удалить действие
function deleteAction(index: number) {
  if (!editingTemplate.value) return;
  editingTemplate.value.actions.splice(index, 1);
  saveTemplates();
}

// Запустить кнопку (показать оверлей)
async function activateButton(template: ButtonTemplate) {
  if (template.actions.length === 0) {
    alert('Добавьте хотя бы одно действие');
    return;
  }

  // Деактивируем все остальные кнопки
  templates.value.forEach(t => t.isActive = false);

  // Активируем эту кнопку
  template.isActive = true;
  saveTemplates();

  try {
    // Создаем оверлейное окно с кнопкой
    await invoke('show_overlay_button', {
      templateId: template.id,
      templateName: template.name
    });
  } catch (error) {
    console.error('Error showing overlay button:', error);
    alert('Ошибка при отображении кнопки: ' + error);
    template.isActive = false;
    saveTemplates();
  }
}

// Деактивировать кнопку
async function deactivateButton(template: ButtonTemplate) {
  template.isActive = false;
  saveTemplates();

  try {
    await invoke('hide_overlay_button');
  } catch (error) {
    console.error('Error hiding overlay button:', error);
  }
}

// Обработка нажатия клавиш для ввода hotkey (только в режиме перехвата)
function handleHotkeyInput(event: KeyboardEvent) {
  if (!isCapturingKeys.value) {
    return; // В режиме текстового ввода не перехватываем
  }

  event.preventDefault();

  const keys = [];
  if (event.ctrlKey) keys.push('Ctrl');
  if (event.altKey) keys.push('Alt');
  if (event.shiftKey) keys.push('Shift');
  if (event.metaKey) keys.push('Super');

  // Добавляем основную клавишу
  if (event.key && !['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) {
    // Обрабатываем специальные клавиши
    const keyName = event.key === ' ' ? 'Space' :
                    event.key === 'Escape' ? 'Esc' :
                    event.key.length === 1 ? event.key.toUpperCase() :
                    event.key;
    keys.push(keyName);
  }

  if (keys.length > 0) {
    hotkey.value = keys.join('+');
  }
}

// Экспорт шаблона
function exportTemplate(template: ButtonTemplate) {
  const dataStr = JSON.stringify(template, null, 2);
  const dataBlob = new Blob([dataStr], { type: 'application/json' });
  const url = URL.createObjectURL(dataBlob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `${template.name}.json`;
  link.click();
  URL.revokeObjectURL(url);
}
</script>

<template>
  <div class="page">
    <h1 class="page-title">Buttons</h1>

    <div class="section">
      <h2 class="section-title">Шаблоны кнопок</h2>
      <p class="section-description">
        Создавайте кнопки с последовательностью действий (клики, горячие клавиши), которые можно запустить и отобразить поверх всех окон.
      </p>

      <div class="template-list">
        <div v-for="template in templates" :key="template.id" class="template-item">
          <div class="template-header">
            <div v-if="isEditingName === template.id" class="template-name-edit">
              <input
                v-model="editingName"
                type="text"
                class="name-input"
                @keyup.enter="saveTemplateName(template)"
                @keyup.esc="cancelEditingName"
                autofocus
              />
              <button @click="saveTemplateName(template)" class="btn btn-save-name">✓</button>
              <button @click="cancelEditingName" class="btn btn-cancel-name">✕</button>
            </div>
            <h3 v-else class="template-name" @dblclick="startEditingName(template)">
              {{ template.name }}
              <span class="edit-hint" @click="startEditingName(template)">✏️</span>
            </h3>
            <div class="template-actions">
              <button
                v-if="!template.isActive"
                @click="activateButton(template)"
                class="btn btn-activate"
              >
                ▶ Запустить
              </button>
              <button
                v-else
                @click="deactivateButton(template)"
                class="btn btn-deactivate"
              >
                ⏹ Остановить
              </button>
              <button @click="editTemplate(template)" class="btn btn-edit">Редактировать</button>
              <button @click="exportTemplate(template)" class="btn btn-export">Экспорт</button>
              <button @click="deleteTemplate(template.id)" class="btn btn-delete">Удалить</button>
            </div>
          </div>
          <div class="template-info">
            <span class="action-count">Действий: {{ template.actions.length }}</span>
            <span v-if="template.isActive" class="active-badge">Активна</span>
          </div>
        </div>
      </div>

      <button v-if="!isCreating" @click="isCreating = true" class="btn btn-new">
        + Создать новый шаблон
      </button>

      <div v-if="isCreating" class="create-form">
        <input
          v-model="newTemplateName"
          type="text"
          placeholder="Название кнопки"
          class="text-input"
          @keyup.enter="createNewTemplate"
        />
        <div class="form-actions">
          <button @click="createNewTemplate" class="btn btn-primary">Создать</button>
          <button @click="isCreating = false; newTemplateName = ''" class="btn btn-cancel">Отмена</button>
        </div>
      </div>
    </div>

    <!-- Редактирование шаблона -->
    <div v-if="editingTemplate" class="section">
      <h2 class="section-title">Редактирование: {{ editingTemplate.name }}</h2>

      <div class="actions-list">
        <div v-for="(action, index) in editingTemplate.actions" :key="index" class="action-item">
          <div v-if="action.type === 'click'" class="action-content">
            <span class="action-type-badge click">Клик</span>
            <span class="action-details">
              x: <input v-model.number="action.x" type="number" class="coord-input" />
              y: <input v-model.number="action.y" type="number" class="coord-input" />
              <select v-model="action.button" class="button-select">
                <option value="left">ЛКМ</option>
                <option value="right">ПКМ</option>
              </select>
            </span>
          </div>
          <div v-else-if="action.type === 'keypress'" class="action-content">
            <span class="action-type-badge keypress">Клавиши</span>
            <span class="action-details">{{ action.keys }}</span>
          </div>
          <button @click="deleteAction(index)" class="btn btn-delete-small">✕</button>
        </div>
      </div>

      <div class="add-action-section">
        <h3>Добавить действие</h3>

        <div class="action-type-selector">
          <button
            :class="['btn', actionType === 'click' ? 'btn-primary' : 'btn-secondary']"
            @click="actionType = 'click'"
          >
            Клик
          </button>
          <button
            :class="['btn', actionType === 'keypress' ? 'btn-primary' : 'btn-secondary']"
            @click="actionType = 'keypress'"
          >
            Горячие клавиши
          </button>
        </div>

        <!-- Добавление клика -->
        <div v-if="actionType === 'click'" class="action-form">
          <button @click="addClickAction" class="btn btn-primary">Добавить клик вручную</button>
          <div class="separator">или</div>
          <button
            v-if="!isRecordingClicks"
            @click="startRecordingClicks"
            class="btn btn-record"
          >
            🔴 Записать клики
          </button>
          <button
            v-else
            @click="stopRecordingClicks"
            class="btn btn-stop"
          >
            ⏹ Завершить запись
          </button>
          <div v-if="isRecordingClicks" class="recording-indicator">
            <span class="recording-dot"></span>
            Запись... (кликайте на экране)
          </div>
        </div>

        <!-- Добавление горячих клавиш -->
        <div v-if="actionType === 'keypress'" class="action-form">
          <p class="hint-text">Введите сочетание клавиш (например: Ctrl+C, Alt+Tab, Print Screen, F12)</p>

          <div class="key-input-mode">
            <button
              :class="['mode-btn', { active: !isCapturingKeys }]"
              @click="isCapturingKeys = false"
            >
              ⌨️ Текстовый ввод
            </button>
            <button
              :class="['mode-btn', { active: isCapturingKeys }]"
              @click="isCapturingKeys = true; hotkey = ''"
            >
              🎯 Перехват клавиш
            </button>
          </div>

          <input
            v-model="hotkey"
            type="text"
            :placeholder="isCapturingKeys ? 'Нажмите клавиши...' : 'Введите текстом (например: Print Screen, F12)'"
            class="text-input"
            @keydown="handleHotkeyInput"
            :readonly="isCapturingKeys"
          />
          <button @click="addKeypressAction" class="btn btn-primary">Добавить</button>
        </div>
      </div>

      <button @click="editingTemplate = null" class="btn btn-cancel">Закрыть редактор</button>
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
  margin-bottom 40px

.section
  background white
  border-radius 12px
  padding 30px
  box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)
  margin-bottom 30px

.section-title
  font-size 24px
  font-weight 600
  color #172B4D
  margin-bottom 10px

.section-description
  font-size 14px
  color #6B778C
  margin-bottom 20px
  line-height 1.6

.template-list
  display flex
  flex-direction column
  gap 16px
  margin-bottom 20px

.template-item
  background #f4f5f7
  border-radius 8px
  padding 20px
  border-left 4px solid #0052cc

.template-header
  display flex
  justify-content space-between
  align-items center
  margin-bottom 12px

.template-name
  font-size 18px
  font-weight 600
  color #172B4D
  display flex
  align-items center
  gap 8px
  cursor pointer
  transition color 0.2s ease

  &:hover
    color #0052cc

  .edit-hint
    font-size 14px
    opacity 0.5
    cursor pointer
    transition opacity 0.2s ease

    &:hover
      opacity 1

.template-name-edit
  display flex
  align-items center
  gap 8px

.name-input
  padding 8px 12px
  border 2px solid #0052cc
  border-radius 6px
  font-size 16px
  font-weight 600
  color #172B4D
  min-width 300px

  &:focus
    outline none
    border-color #0052cc

.btn-save-name
  padding 6px 12px
  background #00875A
  color white
  font-size 16px

  &:hover
    background #006644

.btn-cancel-name
  padding 6px 12px
  background #DE350B
  color white
  font-size 16px

  &:hover
    background #BF2600

.hint-text
  font-size 13px
  color #6B778C
  margin-bottom 8px
  line-height 1.5

.key-input-mode
  display flex
  gap 8px
  margin-bottom 12px

.mode-btn
  flex 1
  padding 10px 16px
  background #DFE1E6
  color #172B4D
  border none
  border-radius 6px
  font-size 14px
  font-weight 500
  cursor pointer
  transition all 0.2s ease

  &:hover
    background #C1C7D0

  &.active
    background #0052cc
    color white

    &:hover
      background #0747a6

.template-actions
  display flex
  gap 8px

.template-info
  display flex
  gap 16px
  align-items center
  font-size 14px
  color #6B778C

.action-count
  font-weight 500

.active-badge
  padding 4px 12px
  background #00875A
  color white
  border-radius 12px
  font-size 12px
  font-weight 600

.btn
  padding 8px 16px
  border none
  border-radius 6px
  font-size 14px
  font-weight 500
  cursor pointer
  transition all 0.2s ease

  &:hover
    transform translateY(-1px)

.btn-new
  width 100%
  padding 12px
  background #0052cc
  color white
  font-size 16px

  &:hover
    background #0747a6

.btn-activate
  background #00875A
  color white

  &:hover
    background #006644

.btn-deactivate
  background #FF5630
  color white

  &:hover
    background #DE350B

.btn-edit
  background #0052cc
  color white

  &:hover
    background #0747a6

.btn-export
  background #6554C0
  color white

  &:hover
    background #5243AA

.btn-delete
  background #DE350B
  color white

  &:hover
    background #BF2600

.btn-delete-small
  padding 4px 8px
  background #DE350B
  color white
  font-size 12px

  &:hover
    background #BF2600

.btn-primary
  background #0052cc
  color white

  &:hover
    background #0747a6

.btn-secondary
  background #DFE1E6
  color #172B4D

  &:hover
    background #C1C7D0

.btn-cancel
  background #DFE1E6
  color #172B4D

  &:hover
    background #C1C7D0

.btn-record
  background #DE350B
  color white

  &:hover
    background #BF2600

.btn-stop
  background #FF5630
  color white
  animation pulse-red 1.5s ease-in-out infinite

  &:hover
    background #DE350B

@keyframes pulse-red
  0%, 100%
    box-shadow 0 0 0 0 rgba(255, 86, 48, 0.4)
  50%
    box-shadow 0 0 0 10px rgba(255, 86, 48, 0)

.create-form
  background #f4f5f7
  border-radius 8px
  padding 20px
  margin-top 16px

.text-input
  width 100%
  padding 12px 16px
  border 2px solid #DFE1E6
  border-radius 8px
  font-size 14px
  font-family inherit
  margin-bottom 12px

  &:focus
    outline none
    border-color #0052cc

.form-actions
  display flex
  gap 12px

.actions-list
  display flex
  flex-direction column
  gap 12px
  margin-bottom 24px

.action-item
  display flex
  justify-content space-between
  align-items center
  background #f4f5f7
  border-radius 8px
  padding 12px 16px

.action-content
  display flex
  align-items center
  gap 12px
  flex 1

.action-type-badge
  padding 4px 12px
  border-radius 12px
  font-size 12px
  font-weight 600
  min-width 80px
  text-align center

  &.click
    background #E3FCEF
    color #006644

  &.keypress
    background #DEEBFF
    color #0052CC

.action-details
  display flex
  align-items center
  gap 8px
  font-size 14px
  color #172B4D

.coord-input
  width 80px
  padding 4px 8px
  border 1px solid #DFE1E6
  border-radius 4px
  font-size 13px

  &:focus
    outline none
    border-color #0052cc

.button-select
  padding 4px 8px
  border 1px solid #DFE1E6
  border-radius 4px
  font-size 13px
  background white
  cursor pointer

  &:focus
    outline none
    border-color #0052cc

.add-action-section
  background #f4f5f7
  border-radius 8px
  padding 20px
  margin-bottom 20px

  h3
    font-size 18px
    font-weight 600
    color #172B4D
    margin-bottom 16px

.action-type-selector
  display flex
  gap 12px
  margin-bottom 16px

.action-form
  display flex
  flex-direction column
  gap 12px

.separator
  text-align center
  color #6B778C
  font-size 14px

.recording-indicator
  display flex
  align-items center
  gap 10px
  padding 12px 16px
  background #FFEBE6
  border-radius 8px
  color #DE350B
  font-weight 500

.recording-dot
  width 12px
  height 12px
  background #DE350B
  border-radius 50%
  animation blink 1s ease-in-out infinite

@keyframes blink
  0%, 100%
    opacity 1
  50%
    opacity 0.3
</style>
