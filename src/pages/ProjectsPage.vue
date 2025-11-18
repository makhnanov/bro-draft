<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface JetBrainsProject {
  ide_name: string;
  ide_version: string;
  project_path: string;
  display_name: string;
  frame_title: string;
  activation_time: string | null;
  exists: boolean;
}

interface IDEGroup {
  name: string;
  version: string;
  projects: JetBrainsProject[];
}

const ideGroups = ref<IDEGroup[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);

// Цвета для разных IDE
const ideColors: Record<string, string> = {
  'Rider': '#C90F5E',
  'PhpStorm': '#B345F1',
  'PyCharm': '#FFD43B',
  'WebStorm': '#00D8FF',
  'GoLand': '#00ADD8',
  'RustRover': '#F74C00',
  'DataGrip': '#22D88F',
  'CLion': '#00D8FF',
  'IntelliJIDEA': '#FE315D',
};

// Иконки для разных IDE (SVG paths)
const ideIcons: Record<string, string> = {
  'Rider': 'M0 0v24h24V0zm7.03 3.75h1.5v5.63h-1.5zm7.5 0h1.5v5.63h-1.5zm-11.25.94h3.75v1.5H3.28zm7.5 0h3.75v1.5h-3.75zm-5.625 3.75h1.88v1.88h-1.88zm7.5 0h1.88v1.88h-1.88z',
  'PhpStorm': 'M0 0v24h24V0zm3.6 3.6h5.4v1.2h-2.1v5.4h-1.2v-5.4h-2.1zm7.2 0h1.2v6.6h-1.2zm-7.2 9.6h5.4v1.2h-5.4zm7.2 0h5.4v1.2h-5.4z',
  'PyCharm': 'M0 0v24h24V0zm3 3h6v1.5h-2.25v5.25h-1.5v-5.25h-2.25zm8.25 0h1.5v6.75h-1.5zm-8.25 9.75h6v1.5h-6zm8.25 0h6v1.5h-6z',
  'WebStorm': 'M0 0v24h24V0zm4.5 4.5h4.5v1.13h-1.69v3.94h-.94v-3.94h-1.87zm5.63 0h.94l1.31 3.75 1.31-3.75h.94l-1.88 5.06h-.75zm-6.13 10.5h4.5v1.13h-4.5zm5.63 0h4.5v1.13h-4.5z',
  'GoLand': 'M0 0v24h24V0zm3.75 3.75h5.63v1.13h-2.25v4.5h-1.13v-4.5h-2.25zm7.5 0h1.13v5.63h-1.13zm-7.5 10.5h5.63v1.13h-5.63zm7.5 0h5.63v1.13h-5.63z',
  'RustRover': 'M0 0v24h24V0zm3.38 3.38h5.62v1.12h-2.25v4.5h-1.12v-4.5h-2.25zm7.5 0h1.12v5.62h-1.12zm-7.5 10.5h5.62v1.12h-5.62zm7.5 0h5.62v1.12h-7.5z',
  'DataGrip': 'M0 0v24h24V0zm3 3h6v1.5h-2.25v5.25h-1.5v-5.25h-2.25zm8.25 0h1.5v6.75h-1.5zm-8.25 9.75h6v1.5h-6zm8.25 0h6v1.5h-6z',
  'CLion': 'M0 0v24h24V0zm3.75 3.75h5.63v1.13h-2.25v4.5h-1.13v-4.5h-2.25zm7.5 0h1.13v5.63h-1.13zm-7.5 10.5h5.63v1.13h-5.63zm7.5 0h5.63v1.13h-5.63z',
};

function getIDEColor(ideName: string): string {
  return ideColors[ideName] || '#0052cc';
}

function getIDEIcon(ideName: string): string {
  return ideIcons[ideName] || ideIcons['IntelliJIDEA'];
}

async function loadProjects() {
  try {
    loading.value = true;
    error.value = null;

    const projects = await invoke<JetBrainsProject[]>('get_jetbrains_projects');

    // Группируем проекты по IDE (только по имени, без версии)
    const grouped = new Map<string, { versions: Map<string, JetBrainsProject[]>, latestVersion: string }>();

    // Сначала группируем по IDE и версиям
    projects.forEach(project => {
      if (!grouped.has(project.ide_name)) {
        grouped.set(project.ide_name, {
          versions: new Map(),
          latestVersion: project.ide_version
        });
      }

      const ideData = grouped.get(project.ide_name)!;

      // Обновляем последнюю версию если текущая новее
      if (compareVersions(project.ide_version, ideData.latestVersion) > 0) {
        ideData.latestVersion = project.ide_version;
      }

      if (!ideData.versions.has(project.ide_version)) {
        ideData.versions.set(project.ide_version, []);
      }
      ideData.versions.get(project.ide_version)!.push(project);
    });

    // Создаём финальный список: только последние версии IDE с объединёнными проектами
    const finalGroups: IDEGroup[] = [];

    grouped.forEach((ideData, ideName) => {
      // Собираем все проекты из всех версий
      const allProjects: JetBrainsProject[] = [];
      const uniquePaths = new Set<string>();

      ideData.versions.forEach((versionProjects) => {
        versionProjects.forEach(project => {
          // Добавляем только уникальные проекты (по пути)
          if (!uniquePaths.has(project.project_path)) {
            uniquePaths.add(project.project_path);
            // Обновляем версию IDE на последнюю
            allProjects.push({
              ...project,
              ide_version: ideData.latestVersion
            });
          }
        });
      });

      // Сортируем проекты по времени активации
      allProjects.sort((a, b) => {
        if (!a.activation_time) return 1;
        if (!b.activation_time) return -1;
        return new Date(b.activation_time).getTime() - new Date(a.activation_time).getTime();
      });

      finalGroups.push({
        name: ideName,
        version: ideData.latestVersion,
        projects: allProjects
      });
    });

    // Сортируем IDE по количеству проектов (от большего к меньшему)
    finalGroups.sort((a, b) => b.projects.length - a.projects.length);

    ideGroups.value = finalGroups;
  } catch (e) {
    error.value = String(e);
    console.error('Failed to load projects:', e);
  } finally {
    loading.value = false;
  }
}

// Функция для сравнения версий (например, "2025.1" > "2024.3")
function compareVersions(v1: string, v2: string): number {
  const parts1 = v1.split('.').map(Number);
  const parts2 = v2.split('.').map(Number);

  for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
    const num1 = parts1[i] || 0;
    const num2 = parts2[i] || 0;

    if (num1 > num2) return 1;
    if (num1 < num2) return -1;
  }

  return 0;
}

async function openProject(project: JetBrainsProject) {
  try {
    await invoke('open_jetbrains_project', {
      projectPath: project.project_path,
      ideName: project.ide_name
    });
  } catch (e) {
    console.error('Failed to open project:', e);
    alert(`Failed to open project: ${e}`);
  }
}

function getProjectName(project: JetBrainsProject): string {
  // Если есть display_name, используем его
  if (project.display_name && project.display_name.trim()) {
    return project.display_name;
  }

  // Иначе берём последнюю часть пути
  const pathParts = project.project_path.split('/');
  const lastName = pathParts[pathParts.length - 1];

  // Убираем расширения файлов (.csproj, .sln и т.д.)
  return lastName.replace(/\.(csproj|sln|iml|idea)$/i, '');
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return 'Never';
  const date = new Date(dateStr);
  const now = new Date();
  const diff = now.getTime() - date.getTime();

  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);

  if (minutes < 1) return 'Just now';
  if (minutes < 60) return `${minutes}m ago`;
  if (hours < 24) return `${hours}h ago`;
  if (days < 7) return `${days}d ago`;

  return date.toLocaleDateString();
}

onMounted(() => {
  loadProjects();
});
</script>

<template>
  <div class="projects-page">
    <div class="page-header">
      <h1 class="page-title">JetBrains Projects</h1>
      <button class="refresh-button" @click="loadProjects" :disabled="loading">
        <svg viewBox="0 0 24 24" class="refresh-icon" :class="{ spinning: loading }">
          <path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z" fill="currentColor"/>
        </svg>
      </button>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <p>Loading projects...</p>
    </div>

    <div v-else-if="error" class="error-state">
      <svg viewBox="0 0 24 24" class="error-icon">
        <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 15h-2v-2h2v2zm0-4h-2V7h2v6z" fill="currentColor"/>
      </svg>
      <p>{{ error }}</p>
      <button class="retry-button" @click="loadProjects">Retry</button>
    </div>

    <div v-else-if="ideGroups.length === 0" class="empty-state">
      <svg viewBox="0 0 24 24" class="empty-icon">
        <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-5 14H7v-2h7v2zm3-4H7v-2h10v2zm0-4H7V7h10v2z" fill="currentColor"/>
      </svg>
      <p>No projects found</p>
    </div>

    <div v-else class="ide-grid">
      <div
        v-for="ide in ideGroups"
        :key="`${ide.name}_${ide.version}`"
        class="ide-card"
        :style="{ '--ide-color': getIDEColor(ide.name) }"
      >
        <div class="ide-header">
          <div class="ide-icon-wrapper">
            <svg viewBox="0 0 24 24" class="ide-icon">
              <path :d="getIDEIcon(ide.name)" fill="white"/>
              <rect x="3" y="14" width="8" height="7" :fill="getIDEColor(ide.name)"/>
              <text x="7" y="19.5" font-size="6" font-weight="bold" fill="white" text-anchor="middle">
                {{ ide.name.substring(0, 2).toUpperCase() }}
              </text>
            </svg>
          </div>
          <div class="ide-info">
            <h2 class="ide-name">{{ ide.name }}</h2>
            <span class="ide-version">v{{ ide.version }}</span>
          </div>
          <div class="project-count">{{ ide.projects.length }}</div>
        </div>

        <div class="projects-list">
          <div
            v-for="project in ide.projects"
            :key="project.project_path"
            class="project-item"
            :class="{ 'project-missing': !project.exists }"
            @click="openProject(project)"
          >
            <div class="project-icon">
              <svg viewBox="0 0 24 24">
                <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z" fill="currentColor"/>
              </svg>
            </div>
            <div class="project-details">
              <div class="project-name">{{ getProjectName(project) }}</div>
              <div class="project-path">{{ project.project_path }}</div>
              <div class="project-meta">
                <span class="project-time">{{ formatDate(project.activation_time) }}</span>
                <span v-if="!project.exists" class="project-status">Missing</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="stylus">
.projects-page
  padding 30px
  max-width 100%
  overflow-x hidden
  overflow-y auto
  height 100vh

.page-header
  display flex
  justify-content space-between
  align-items center
  margin-bottom 30px

.page-title
  font-size 32px
  font-weight 700
  color #0052cc
  margin 0

.refresh-button
  background #0052cc
  color white
  border none
  border-radius 8px
  padding 10px 16px
  cursor pointer
  display flex
  align-items center
  gap 8px
  transition all 0.2s ease
  font-size 14px
  font-weight 600

  &:hover
    background #0747a6
    transform translateY(-2px)
    box-shadow 0 4px 12px rgba(5, 82, 204, 0.3)

  &:active
    transform translateY(0)

  &:disabled
    opacity 0.6
    cursor not-allowed
    transform none

.refresh-icon
  width 20px
  height 20px

  &.spinning
    animation spin 1s linear infinite

@keyframes spin
  from
    transform rotate(0deg)
  to
    transform rotate(360deg)

.loading-state, .error-state, .empty-state
  display flex
  flex-direction column
  align-items center
  justify-content center
  min-height 400px
  color #666

  svg
    width 64px
    height 64px
    margin-bottom 20px
    opacity 0.5

  p
    font-size 18px
    margin 0

.spinner
  width 48px
  height 48px
  border 4px solid rgba(5, 82, 204, 0.1)
  border-top-color #0052cc
  border-radius 50%
  animation spin 1s linear infinite
  margin-bottom 20px

.retry-button
  margin-top 20px
  padding 10px 24px
  background #0052cc
  color white
  border none
  border-radius 8px
  cursor pointer
  font-size 16px
  font-weight 600
  transition all 0.2s ease

  &:hover
    background #0747a6

.ide-grid
  display grid
  grid-template-columns repeat(auto-fill, minmax(350px, 1fr))
  gap 24px
  margin-bottom 30px

.ide-card
  background white
  border-radius 12px
  overflow hidden
  box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)
  transition all 0.3s ease
  border-top 4px solid var(--ide-color)

  &:hover
    box-shadow 0 8px 24px rgba(0, 0, 0, 0.15)
    transform translateY(-4px)

.ide-header
  padding 20px
  background var(--ide-color)
  color white
  display flex
  align-items center
  gap 15px
  position relative
  overflow hidden

  &::before
    content ''
    position absolute
    top 0
    left 0
    right 0
    bottom 0
    background linear-gradient(135deg, transparent 0%, rgba(0, 0, 0, 0.3) 100%)
    pointer-events none

.ide-icon-wrapper
  flex-shrink 0
  width 48px
  height 48px
  background rgba(255, 255, 255, 0.2)
  border-radius 8px
  display flex
  align-items center
  justify-content center
  position relative
  z-index 1

.ide-icon
  width 40px
  height 40px

.ide-info
  flex 1
  position relative
  z-index 1

.ide-name
  margin 0
  font-size 20px
  font-weight 700
  color white

.ide-version
  font-size 13px
  opacity 0.9
  color rgba(255, 255, 255, 0.9)

.project-count
  background rgba(255, 255, 255, 0.2)
  padding 6px 12px
  border-radius 12px
  font-size 14px
  font-weight 700
  position relative
  z-index 1

.projects-list
  max-height 400px
  overflow-y auto

  &::-webkit-scrollbar
    width 8px

  &::-webkit-scrollbar-track
    background #f1f1f1

  &::-webkit-scrollbar-thumb
    background #888
    border-radius 4px

    &:hover
      background #555

.project-item
  padding 16px 20px
  border-bottom 1px solid #e8e8e8
  cursor pointer
  transition all 0.2s ease
  display flex
  gap 12px
  align-items flex-start

  &:last-child
    border-bottom none

  &:hover
    background #f8f9fa

  &.project-missing
    opacity 0.5

    .project-icon
      svg
        fill #dc3545

.project-icon
  flex-shrink 0
  width 32px
  height 32px
  display flex
  align-items center
  justify-content center

  svg
    width 24px
    height 24px
    fill var(--ide-color)

.project-details
  flex 1
  min-width 0

.project-name
  font-size 15px
  font-weight 600
  color #333
  margin-bottom 4px
  overflow hidden
  text-overflow ellipsis
  white-space nowrap

.project-path
  font-size 12px
  color #888
  font-family monospace
  margin-bottom 6px
  overflow hidden
  text-overflow ellipsis
  white-space nowrap

.project-meta
  display flex
  gap 12px
  align-items center

.project-time
  font-size 11px
  color #999

.project-status
  font-size 11px
  color #dc3545
  background rgba(220, 53, 69, 0.1)
  padding 2px 8px
  border-radius 4px
  font-weight 600

@media (prefers-color-scheme: dark)
  .ide-card
    background #2f2f2f

  .project-item
    border-bottom-color #444

    &:hover
      background #3a3a3a

  .project-name
    color #f6f6f6

  .project-path
    color #aaa

  .projects-list
    &::-webkit-scrollbar-track
      background #2f2f2f

    &::-webkit-scrollbar-thumb
      background #555

      &:hover
        background #777
</style>
