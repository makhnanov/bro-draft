<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

interface SideButton {
  id: string;
  name: string;
  iconPath: string;
  command: string;
  edge: 'left' | 'right' | 'top' | 'bottom';
  position: number;
  isActive: boolean;
  lastX?: number;
  lastY?: number;
}

const buttons = ref<SideButton[]>([]);
const isCreating = ref(false);
const editingButton = ref<SideButton | null>(null);

// Form fields
const formName = ref('');
const formIconPath = ref('');
const formCommand = ref('');
const formEdge = ref<'left' | 'right' | 'top' | 'bottom'>('right');
const formPosition = ref(50);

// Cache of icon data URIs (iconPath -> data URI)
const iconCache = ref<Record<string, string>>({});
const formIconDataUri = ref('');

async function loadIconDataUri(path: string): Promise<string> {
  if (!path) return '';
  if (iconCache.value[path]) return iconCache.value[path];
  try {
    const dataUri = await invoke<string>('read_icon_base64', { path });
    iconCache.value[path] = dataUri;
    return dataUri;
  } catch (e) {
    console.error('Failed to load icon:', e);
    return '';
  }
}

async function loadAllIcons() {
  for (const btn of buttons.value) {
    if (btn.iconPath) {
      await loadIconDataUri(btn.iconPath);
    }
  }
}

onMounted(async () => {
  try {
    const saved = await invoke<string>('load_side_buttons');
    if (saved) {
      buttons.value = JSON.parse(saved);
      await loadAllIcons();
    }
  } catch (error) {
    console.error('Failed to load side buttons:', error);
  }
});

async function saveButtons() {
  try {
    await invoke('save_side_buttons', { json: JSON.stringify(buttons.value) });
  } catch (error) {
    console.error('Failed to save side buttons:', error);
  }
}

async function selectIcon() {
  const file = await open({
    multiple: false,
    directory: false,
    filters: [{
      name: 'Images',
      extensions: ['png', 'jpg', 'jpeg', 'svg', 'ico', 'webp', 'gif']
    }]
  });
  if (file) {
    formIconPath.value = file as string;
    formIconDataUri.value = await loadIconDataUri(formIconPath.value);
  }
}

function resetForm() {
  formName.value = '';
  formIconPath.value = '';
  formCommand.value = '';
  formEdge.value = 'right';
  formPosition.value = 50;
}

function startCreating() {
  resetForm();
  editingButton.value = null;
  isCreating.value = true;
}

async function startEditing(button: SideButton) {
  formName.value = button.name;
  formIconPath.value = button.iconPath;
  formCommand.value = button.command;
  formEdge.value = button.edge;
  formPosition.value = button.position;
  editingButton.value = button;
  isCreating.value = true;
  if (button.iconPath) {
    formIconDataUri.value = await loadIconDataUri(button.iconPath);
  } else {
    formIconDataUri.value = '';
  }
}

function cancelForm() {
  isCreating.value = false;
  editingButton.value = null;
  resetForm();
}

function saveForm() {
  if (!formName.value.trim()) {
    alert('Enter a button name');
    return;
  }
  if (!formCommand.value.trim()) {
    alert('Enter a command to launch');
    return;
  }

  if (editingButton.value) {
    // Update existing
    editingButton.value.name = formName.value.trim();
    editingButton.value.iconPath = formIconPath.value;
    editingButton.value.command = formCommand.value.trim();
    editingButton.value.edge = formEdge.value;
    editingButton.value.position = formPosition.value;
  } else {
    // Create new
    const btn: SideButton = {
      id: Date.now().toString(),
      name: formName.value.trim(),
      iconPath: formIconPath.value,
      command: formCommand.value.trim(),
      edge: formEdge.value,
      position: formPosition.value,
      isActive: false
    };
    buttons.value.push(btn);
  }

  isCreating.value = false;
  editingButton.value = null;
  resetForm();
  saveButtons();
}

function deleteButton(id: string) {
  if (!confirm('Delete this side button?')) return;
  const btn = buttons.value.find(b => b.id === id);
  if (btn?.isActive) {
    deactivateButton(btn);
  }
  buttons.value = buttons.value.filter(b => b.id !== id);
  saveButtons();
}

async function activateButton(button: SideButton) {
  try {
    await invoke('show_side_button', {
      id: button.id,
      name: button.name,
      iconPath: button.iconPath,
      command: button.command,
      edge: button.edge,
      position: button.position,
      lastX: button.lastX ?? null,
      lastY: button.lastY ?? null,
    });
    button.isActive = true;
    saveButtons();
  } catch (error) {
    console.error('Error showing side button:', error);
    alert('Error activating button: ' + error);
  }
}

async function deactivateButton(button: SideButton) {
  try {
    await invoke('hide_side_button', { id: button.id });
    button.isActive = false;
    saveButtons();
  } catch (error) {
    console.error('Error hiding side button:', error);
  }
}

function edgeLabel(edge: string): string {
  const labels: Record<string, string> = {
    left: 'Left',
    right: 'Right',
    top: 'Top',
    bottom: 'Bottom'
  };
  return labels[edge] || edge;
}
</script>

<template>
  <div class="page">
    <h1 class="page-title">Side Menu</h1>

    <div class="section">
      <h2 class="section-title">Floating Edge Buttons</h2>
      <p class="section-description">
        Create floating icon buttons that snap to screen edges. Click them to launch applications. They auto-hide when the cursor leaves.
      </p>

      <div class="button-list">
        <div v-for="btn in buttons" :key="btn.id" class="button-item">
          <div class="button-header">
            <div class="button-info-row">
              <div v-if="btn.iconPath && iconCache[btn.iconPath]" class="button-icon-preview">
                <img :src="iconCache[btn.iconPath]" alt="icon" />
              </div>
              <div v-else class="button-icon-placeholder">?</div>
              <div class="button-details">
                <h3 class="button-name">{{ btn.name }}</h3>
                <span class="button-meta">{{ btn.command }} &middot; {{ edgeLabel(btn.edge) }} edge &middot; {{ btn.position }}%</span>
              </div>
            </div>
            <div class="button-actions">
              <button
                v-if="!btn.isActive"
                @click="activateButton(btn)"
                class="btn btn-activate"
              >
                Show
              </button>
              <button
                v-else
                @click="deactivateButton(btn)"
                class="btn btn-deactivate"
              >
                Hide
              </button>
              <button @click="startEditing(btn)" class="btn btn-edit">Edit</button>
              <button @click="deleteButton(btn.id)" class="btn btn-delete">Delete</button>
            </div>
          </div>
        </div>
      </div>

      <button v-if="!isCreating" @click="startCreating" class="btn btn-new">
        + Create Side Button
      </button>

      <div v-if="isCreating" class="create-form">
        <h3 class="form-title">{{ editingButton ? 'Edit' : 'New' }} Side Button</h3>

        <label class="form-label">Name</label>
        <input
          v-model="formName"
          type="text"
          placeholder="Button name"
          class="text-input"
        />

        <label class="form-label">Icon Image</label>
        <div class="icon-select-row">
          <input
            v-model="formIconPath"
            type="text"
            placeholder="Path to icon image (optional)"
            class="text-input icon-input"
          />
          <button @click="selectIcon" class="btn btn-secondary">Browse...</button>
        </div>
        <div v-if="formIconDataUri" class="icon-preview-large">
          <img :src="formIconDataUri" alt="preview" />
        </div>

        <label class="form-label">Command</label>
        <input
          v-model="formCommand"
          type="text"
          placeholder="e.g. firefox, code, /usr/bin/gimp"
          class="text-input"
        />

        <label class="form-label">Screen Edge</label>
        <div class="edge-selector">
          <button
            v-for="edge in (['left', 'right', 'top', 'bottom'] as const)"
            :key="edge"
            :class="['btn', formEdge === edge ? 'btn-primary' : 'btn-secondary']"
            @click="formEdge = edge"
          >
            {{ edgeLabel(edge) }}
          </button>
        </div>

        <label class="form-label">Position along edge ({{ formPosition }}%)</label>
        <input
          v-model.number="formPosition"
          type="range"
          min="0"
          max="100"
          class="range-input"
        />

        <div class="form-actions">
          <button @click="saveForm" class="btn btn-primary">{{ editingButton ? 'Save' : 'Create' }}</button>
          <button @click="cancelForm" class="btn btn-cancel">Cancel</button>
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
  width 100%
  box-sizing border-box

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

.button-list
  display flex
  flex-direction column
  gap 16px
  margin-bottom 20px

.button-item
  background #f4f5f7
  border-radius 8px
  padding 20px
  border-left 4px solid #0052cc
  position relative

.button-header
  display flex
  justify-content space-between
  align-items center

.button-info-row
  display flex
  align-items center
  gap 14px

.button-icon-preview
  width 40px
  height 40px
  border-radius 8px
  overflow hidden
  flex-shrink 0
  background #fff
  display flex
  align-items center
  justify-content center

  img
    width 100%
    height 100%
    object-fit contain

.button-icon-placeholder
  width 40px
  height 40px
  border-radius 8px
  background #DFE1E6
  display flex
  align-items center
  justify-content center
  font-size 18px
  color #6B778C
  flex-shrink 0

.button-details
  display flex
  flex-direction column
  gap 4px

.button-name
  font-size 18px
  font-weight 600
  color #172B4D

.button-meta
  font-size 13px
  color #6B778C

.button-actions
  display flex
  gap 8px

.active-badge
  display inline-block
  margin-top 10px
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

.btn-delete
  background #DE350B
  color white

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

.create-form
  background #f4f5f7
  border-radius 8px
  padding 24px
  margin-top 16px

.form-title
  font-size 18px
  font-weight 600
  color #172B4D
  margin-bottom 16px

.form-label
  display block
  font-size 13px
  font-weight 600
  color #6B778C
  margin-bottom 6px
  margin-top 14px

  &:first-of-type
    margin-top 0

.text-input
  width 100%
  padding 10px 14px
  border 2px solid #DFE1E6
  border-radius 8px
  font-size 14px
  font-family inherit

  &:focus
    outline none
    border-color #0052cc

.icon-select-row
  display flex
  gap 8px
  align-items center

  .icon-input
    flex 1

.icon-preview-large
  margin-top 10px
  width 64px
  height 64px
  border-radius 8px
  overflow hidden
  background #fff
  border 2px solid #DFE1E6

  img
    width 100%
    height 100%
    object-fit contain

.edge-selector
  display flex
  gap 8px

.range-input
  width 100%
  margin-top 4px

.form-actions
  display flex
  gap 12px
  margin-top 20px
</style>
