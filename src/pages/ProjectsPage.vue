<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
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

interface RecentProjectClick {
  project_path: string;
  clicked_at: number;
}

const ideGroups = ref<IDEGroup[]>([]);
const allIdeGroups = ref<IDEGroup[]>([]); // Все группы без фильтрации
const recentProjects = ref<JetBrainsProject[]>([]); // Последние открытые проекты (по кликам в приложении)
const loading = ref(true);
const error = ref<string | null>(null);
const searchQuery = ref('');

// Hidden projects
const hiddenProjects = ref<Set<string>>(new Set());
const showHiddenForIDE = ref<Set<string>>(new Set()); // Which IDEs are showing hidden projects
const contextMenu = ref<{ show: boolean; x: number; y: number; project: JetBrainsProject | null }>({
  show: false,
  x: 0,
  y: 0,
  project: null
});

// Check if project is hidden
function isProjectHidden(projectPath: string): boolean {
  return hiddenProjects.value.has(projectPath);
}

// Get hidden projects count for an IDE
function getHiddenCount(ideName: string): number {
  const ideGroup = allIdeGroups.value.find(g => g.name === ideName);
  if (!ideGroup) return 0;
  return ideGroup.projects.filter(p => hiddenProjects.value.has(p.project_path)).length;
}

// Get fully hidden IDEs (all projects hidden)
function getFullyHiddenIDEs(): IDEGroup[] {
  return allIdeGroups.value.filter(ideGroup => {
    const hiddenCount = ideGroup.projects.filter(p => hiddenProjects.value.has(p.project_path)).length;
    return hiddenCount === ideGroup.projects.length && hiddenCount > 0;
  });
}

// Show all projects for a fully hidden IDE
function showHiddenIDE(ideName: string) {
  showHiddenForIDE.value.add(ideName);
  filterProjects();
}

// Toggle show hidden for IDE
function toggleShowHidden(ideName: string) {
  if (showHiddenForIDE.value.has(ideName)) {
    showHiddenForIDE.value.delete(ideName);
  } else {
    showHiddenForIDE.value.add(ideName);
  }
  filterProjects();
}

// Unhide project
function unhideProject() {
  if (contextMenu.value.project) {
    hiddenProjects.value.delete(contextMenu.value.project.project_path);
    saveHiddenProjects();
    filterProjects();
  }
  hideContextMenu();
}

// Load hidden projects from localStorage
function loadHiddenProjects() {
  const saved = localStorage.getItem('hiddenProjects');
  if (saved) {
    hiddenProjects.value = new Set(JSON.parse(saved));
  }
}

// Save hidden projects to localStorage
function saveHiddenProjects() {
  localStorage.setItem('hiddenProjects', JSON.stringify([...hiddenProjects.value]));
}

// Load recent project clicks from localStorage
function loadRecentClicks(): RecentProjectClick[] {
  const saved = localStorage.getItem('recentProjectClicks');
  if (saved) {
    return JSON.parse(saved);
  }
  return [];
}

// Save recent project click
function saveProjectClick(projectPath: string) {
  const clicks = loadRecentClicks();

  // Удаляем старую запись если есть
  const filtered = clicks.filter(c => c.project_path !== projectPath);

  // Добавляем новую запись в начало
  filtered.unshift({
    project_path: projectPath,
    clicked_at: Date.now()
  });

  // Сохраняем только последние 50 кликов
  const limited = filtered.slice(0, 50);
  localStorage.setItem('recentProjectClicks', JSON.stringify(limited));
}

// Update recent projects list based on clicks
function updateRecentProjects() {
  const clicks = loadRecentClicks();

  // Создаем мапу всех доступных проектов для быстрого поиска
  const projectsMap = new Map<string, JetBrainsProject>();
  allIdeGroups.value.forEach(group => {
    group.projects.forEach(project => {
      projectsMap.set(project.project_path, project);
    });
  });

  // Формируем список недавно открытых проектов на основе кликов
  const recent: JetBrainsProject[] = [];
  for (const click of clicks) {
    const project = projectsMap.get(click.project_path);
    if (project && recent.length < 5) {
      recent.push(project);
    }
  }

  recentProjects.value = recent;
}

// Show context menu
function showContextMenu(event: MouseEvent, project: JetBrainsProject) {
  event.preventDefault();
  contextMenu.value = {
    show: true,
    x: event.clientX,
    y: event.clientY,
    project
  };
}

// Hide context menu
function hideContextMenu() {
  contextMenu.value.show = false;
}

// Hide project
function hideProject() {
  if (contextMenu.value.project) {
    hiddenProjects.value.add(contextMenu.value.project.project_path);
    saveHiddenProjects();
    filterProjects();
  }
  hideContextMenu();
}


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

    allIdeGroups.value = finalGroups;
    ideGroups.value = finalGroups;

    // Обновляем список недавно открытых проектов
    updateRecentProjects();
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
    // Сохраняем клик перед открытием
    saveProjectClick(project.project_path);

    await invoke('open_jetbrains_project', {
      projectPath: project.project_path,
      ideName: project.ide_name
    });

    // Обновляем список после сохранения клика
    updateRecentProjects();
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

// Функция поиска по проектам
function filterProjects() {
  const query = searchQuery.value.toLowerCase().trim();

  // Фильтруем проекты в каждой IDE
  const filtered: IDEGroup[] = [];

  allIdeGroups.value.forEach(ideGroup => {
    const showingHidden = showHiddenForIDE.value.has(ideGroup.name);

    const matchingProjects = ideGroup.projects.filter(project => {
      const isHidden = hiddenProjects.value.has(project.project_path);

      // Если проект скрыт и не показываем скрытые для этой IDE - исключаем
      if (isHidden && !showingHidden) {
        return false;
      }

      // Если нет поискового запроса, показываем проект
      if (!query) {
        return true;
      }

      const projectName = getProjectName(project).toLowerCase();
      const projectPath = project.project_path.toLowerCase();

      return projectName.includes(query) || projectPath.includes(query);
    });

    // Добавляем IDE только если есть подходящие проекты
    if (matchingProjects.length > 0) {
      filtered.push({
        ...ideGroup,
        projects: matchingProjects
      });
    }
  });

  ideGroups.value = filtered;
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

// Автоматическое обновление
let autoRefreshInterval: number | null = null;

function handleWindowFocus() {
  loadProjects();
}

onMounted(() => {
  loadHiddenProjects();
  loadProjects();
  document.addEventListener('click', hideContextMenu);

  // Обновляем при возвращении фокуса на окно
  window.addEventListener('focus', handleWindowFocus);

  // Автоматическое обновление каждые 30 секунд
  autoRefreshInterval = window.setInterval(() => {
    loadProjects();
  }, 30000);
});

onUnmounted(() => {
  document.removeEventListener('click', hideContextMenu);
  window.removeEventListener('focus', handleWindowFocus);

  if (autoRefreshInterval !== null) {
    clearInterval(autoRefreshInterval);
  }
});
</script>

<template>
  <div class="projects-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">JetBrains Projects</h1>
        <div class="search-box">
          <svg viewBox="0 0 24 24" class="search-icon">
            <path d="M15.5 14h-.79l-.28-.27A6.471 6.471 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z" fill="currentColor"/>
          </svg>
          <input
            v-model="searchQuery"
            @input="filterProjects"
            type="text"
            placeholder="Search projects..."
            class="search-input"
          />
          <button v-if="searchQuery" @click="searchQuery = ''; filterProjects()" class="clear-search">
            <svg viewBox="0 0 24 24" class="clear-icon">
              <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" fill="currentColor"/>
            </svg>
          </button>
        </div>
      </div>
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

    <template v-else>
      <!-- Recent Projects Section -->
      <div v-if="recentProjects.length > 0" class="recent-section">
        <div class="recent-header">
          <svg viewBox="0 0 24 24" class="recent-icon">
            <path d="M13 3c-4.97 0-9 4.03-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42C8.27 19.99 10.51 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z" fill="currentColor"/>
          </svg>
          <h2 class="recent-title">Recently Opened</h2>
        </div>

        <div class="recent-grid">
          <div
            v-for="project in recentProjects"
            :key="`recent_${project.project_path}`"
            class="recent-project-card"
            :style="{ '--ide-color': getIDEColor(project.ide_name) }"
            @click="openProject(project)"
            @contextmenu="showContextMenu($event, project)"
          >
            <div class="recent-project-header">
              <div class="recent-ide-badge">
                <img v-if="project.ide_name === 'PhpStorm'" src="/PhpStorm.svg" class="recent-ide-icon" alt="PhpStorm" />
                <img v-else-if="project.ide_name === 'WebStorm'" src="/WebStorm.svg" class="recent-ide-icon" alt="WebStorm" />
                <img v-else-if="project.ide_name === 'GoLand'" src="/GoLand.svg" class="recent-ide-icon" alt="GoLand" />
                <img v-else-if="project.ide_name === 'PyCharm'" src="/PyCharm.svg" class="recent-ide-icon" alt="PyCharm" />
                <img v-else-if="project.ide_name === 'DataGrip'" src="/DataGrip.svg" class="recent-ide-icon" alt="DataGrip" />
                <img v-else-if="project.ide_name === 'CLion'" src="/CLion.svg" class="recent-ide-icon" alt="CLion" />
                <svg v-else viewBox="0 0 24 24" class="recent-ide-icon-svg">
                  <rect width="24" height="24" :fill="getIDEColor(project.ide_name)" rx="4"/>
                  <text x="12" y="16" font-size="10" font-weight="bold" fill="white" text-anchor="middle">
                    {{ project.ide_name.substring(0, 2).toUpperCase() }}
                  </text>
                </svg>
                <span class="recent-ide-name">{{ project.ide_name }}</span>
              </div>
              <div class="recent-time">{{ formatDate(project.activation_time) }}</div>
            </div>
            <div class="recent-project-body">
              <div class="recent-project-icon">
                <svg viewBox="0 0 24 24">
                  <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z" fill="currentColor"/>
                </svg>
              </div>
              <div class="recent-project-info">
                <div class="recent-project-name">{{ getProjectName(project) }}</div>
                <div class="recent-project-path">{{ project.project_path }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="ide-grid">
      <div
        v-for="ide in ideGroups"
        :key="`${ide.name}_${ide.version}`"
        class="ide-card"
        :style="{ '--ide-color': getIDEColor(ide.name) }"
      >
        <div class="ide-header">
          <div class="ide-icon-wrapper">
            <img v-if="ide.name === 'PhpStorm'" src="/PhpStorm.svg" class="ide-icon-img" alt="PhpStorm" />
            <img v-else-if="ide.name === 'WebStorm'" src="/WebStorm.svg" class="ide-icon-img" alt="WebStorm" />
            <img v-else-if="ide.name === 'GoLand'" src="/GoLand.svg" class="ide-icon-img" alt="GoLand" />
            <img v-else-if="ide.name === 'PyCharm'" src="/PyCharm.svg" class="ide-icon-img" alt="PyCharm" />
            <img v-else-if="ide.name === 'DataGrip'" src="/DataGrip.svg" class="ide-icon-img" alt="DataGrip" />
            <img v-else-if="ide.name === 'CLion'" src="/CLion.svg" class="ide-icon-img" alt="CLion" />
            <svg v-else viewBox="0 0 24 24" class="ide-icon">
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
          <button
            v-if="getHiddenCount(ide.name) > 0"
            class="toggle-hidden-btn"
            :class="{ active: showHiddenForIDE.has(ide.name) }"
            @click.stop="toggleShowHidden(ide.name)"
            :title="showHiddenForIDE.has(ide.name) ? 'Hide hidden projects' : `Show ${getHiddenCount(ide.name)} hidden`"
          >
            <svg viewBox="0 0 24 24" class="eye-icon">
              <path v-if="showHiddenForIDE.has(ide.name)" d="M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zM12 17c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm0-8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z" fill="currentColor"/>
              <path v-else d="M12 7c2.76 0 5 2.24 5 5 0 .65-.13 1.26-.36 1.83l2.92 2.92c1.51-1.26 2.7-2.89 3.43-4.75-1.73-4.39-6-7.5-11-7.5-1.4 0-2.74.25-3.98.7l2.16 2.16C10.74 7.13 11.35 7 12 7zM2 4.27l2.28 2.28.46.46C3.08 8.3 1.78 10.02 1 12c1.73 4.39 6 7.5 11 7.5 1.55 0 3.03-.3 4.38-.84l.42.42L19.73 22 21 20.73 3.27 3 2 4.27zM7.53 9.8l1.55 1.55c-.05.21-.08.43-.08.65 0 1.66 1.34 3 3 3 .22 0 .44-.03.65-.08l1.55 1.55c-.67.33-1.41.53-2.2.53-2.76 0-5-2.24-5-5 0-.79.2-1.53.53-2.2zm4.31-.78l3.15 3.15.02-.16c0-1.66-1.34-3-3-3l-.17.01z" fill="currentColor"/>
            </svg>
          </button>
        </div>

        <div class="projects-list">
          <div
            v-for="project in ide.projects"
            :key="project.project_path"
            class="project-item"
            :class="{ 'project-missing': !project.exists, 'project-hidden': isProjectHidden(project.project_path) }"
            @click="openProject(project)"
            @contextmenu="showContextMenu($event, project)"
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
    </template>

    <!-- Hidden IDEs section -->
    <div v-if="getFullyHiddenIDEs().length > 0" class="hidden-ides-section">
      <div class="hidden-ides-header">
        <svg viewBox="0 0 24 24" class="hidden-ides-icon">
          <path d="M12 7c2.76 0 5 2.24 5 5 0 .65-.13 1.26-.36 1.83l2.92 2.92c1.51-1.26 2.7-2.89 3.43-4.75-1.73-4.39-6-7.5-11-7.5-1.4 0-2.74.25-3.98.7l2.16 2.16C10.74 7.13 11.35 7 12 7zM2 4.27l2.28 2.28.46.46C3.08 8.3 1.78 10.02 1 12c1.73 4.39 6 7.5 11 7.5 1.55 0 3.03-.3 4.38-.84l.42.42L19.73 22 21 20.73 3.27 3 2 4.27zM7.53 9.8l1.55 1.55c-.05.21-.08.43-.08.65 0 1.66 1.34 3 3 3 .22 0 .44-.03.65-.08l1.55 1.55c-.67.33-1.41.53-2.2.53-2.76 0-5-2.24-5-5 0-.79.2-1.53.53-2.2zm4.31-.78l3.15 3.15.02-.16c0-1.66-1.34-3-3-3l-.17.01z" fill="currentColor"/>
        </svg>
        <span>{{ getFullyHiddenIDEs().length }} hidden IDE(s)</span>
      </div>
      <div class="hidden-ides-list">
        <button
          v-for="ide in getFullyHiddenIDEs()"
          :key="ide.name"
          class="hidden-ide-btn"
          :style="{ '--ide-color': getIDEColor(ide.name) }"
          @click="showHiddenIDE(ide.name)"
        >
          {{ ide.name }} ({{ ide.projects.length }})
        </button>
      </div>
    </div>

    <!-- Context menu -->
    <div
      v-if="contextMenu.show"
      class="context-menu"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }"
    >
      <div v-if="contextMenu.project && isProjectHidden(contextMenu.project.project_path)" class="context-menu-item" @click="unhideProject">
        <svg viewBox="0 0 24 24" class="context-menu-icon">
          <path d="M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zM12 17c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm0-8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z" fill="currentColor"/>
        </svg>
        Show project
      </div>
      <div v-else class="context-menu-item" @click="hideProject">
        <svg viewBox="0 0 24 24" class="context-menu-icon">
          <path d="M12 7c2.76 0 5 2.24 5 5 0 .65-.13 1.26-.36 1.83l2.92 2.92c1.51-1.26 2.7-2.89 3.43-4.75-1.73-4.39-6-7.5-11-7.5-1.4 0-2.74.25-3.98.7l2.16 2.16C10.74 7.13 11.35 7 12 7zM2 4.27l2.28 2.28.46.46C3.08 8.3 1.78 10.02 1 12c1.73 4.39 6 7.5 11 7.5 1.55 0 3.03-.3 4.38-.84l.42.42L19.73 22 21 20.73 3.27 3 2 4.27zM7.53 9.8l1.55 1.55c-.05.21-.08.43-.08.65 0 1.66 1.34 3 3 3 .22 0 .44-.03.65-.08l1.55 1.55c-.67.33-1.41.53-2.2.53-2.76 0-5-2.24-5-5 0-.79.2-1.53.53-2.2zm4.31-.78l3.15 3.15.02-.16c0-1.66-1.34-3-3-3l-.17.01z" fill="currentColor"/>
        </svg>
        Hide project
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
  gap 20px

.header-left
  flex 1
  display flex
  flex-direction column
  gap 16px

.page-title
  font-size 32px
  font-weight 700
  color #0052cc
  margin 0

.search-box
  position relative
  display flex
  align-items center
  max-width 500px
  width 100%

.search-icon
  position absolute
  left 12px
  width 20px
  height 20px
  color #888
  pointer-events none

.search-input
  width 100%
  padding 10px 40px 10px 40px
  border 2px solid #e0e0e0
  border-radius 8px
  font-size 14px
  transition all 0.2s ease
  outline none

  &:focus
    border-color #0052cc
    box-shadow 0 0 0 3px rgba(5, 82, 204, 0.1)

  &::placeholder
    color #aaa

.clear-search
  position absolute
  right 8px
  background transparent
  border none
  cursor pointer
  padding 4px
  display flex
  align-items center
  justify-content center
  border-radius 4px
  transition background 0.2s ease

  &:hover
    background rgba(0, 0, 0, 0.05)

.clear-icon
  width 18px
  height 18px
  color #888

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

// Recent Projects Section
.recent-section
  margin-bottom 40px
  background linear-gradient(135deg, #667eea 0%, #764ba2 100%)
  border-radius 16px
  padding 30px
  box-shadow 0 10px 40px rgba(102, 126, 234, 0.3)

.recent-header
  display flex
  align-items center
  gap 12px
  margin-bottom 24px

.recent-icon
  width 28px
  height 28px
  color white

.recent-title
  font-size 24px
  font-weight 700
  color white
  margin 0

.recent-grid
  display grid
  grid-template-columns repeat(auto-fill, minmax(280px, 1fr))
  gap 16px

.recent-project-card
  background white
  border-radius 12px
  padding 16px
  cursor pointer
  transition all 0.3s ease
  border-left 4px solid var(--ide-color)
  box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)

  &:hover
    transform translateY(-4px)
    box-shadow 0 8px 24px rgba(0, 0, 0, 0.2)

.recent-project-header
  display flex
  justify-content space-between
  align-items center
  margin-bottom 12px
  padding-bottom 12px
  border-bottom 1px solid #e8e8e8

.recent-ide-badge
  display flex
  align-items center
  gap 8px

.recent-ide-icon
  width 24px
  height 24px
  object-fit contain

.recent-ide-icon-svg
  width 24px
  height 24px

.recent-ide-name
  font-size 13px
  font-weight 600
  color var(--ide-color)

.recent-time
  font-size 11px
  color #999
  font-weight 500

.recent-project-body
  display flex
  gap 12px
  align-items flex-start

.recent-project-icon
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

.recent-project-info
  flex 1
  min-width 0

.recent-project-name
  font-size 15px
  font-weight 600
  color #333
  margin-bottom 4px
  overflow hidden
  text-overflow ellipsis
  white-space nowrap

.recent-project-path
  font-size 11px
  color #888
  font-family monospace
  overflow hidden
  text-overflow ellipsis
  white-space nowrap

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

.ide-icon-img
  width 40px
  height 40px
  object-fit contain

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

.toggle-hidden-btn
  background rgba(255, 255, 255, 0.2)
  border none
  border-radius 8px
  padding 6px
  cursor pointer
  display flex
  align-items center
  justify-content center
  position relative
  z-index 1
  transition all 0.2s ease
  margin-left 8px

  &:hover
    background rgba(255, 255, 255, 0.3)

  &.active
    background rgba(255, 255, 255, 0.4)

  .eye-icon
    width 20px
    height 20px
    color white

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

  &.project-hidden
    opacity 0.5
    background rgba(108, 117, 125, 0.1)

    .project-name
      text-decoration line-through

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

// Context menu styles
.context-menu
  position fixed
  background white
  border-radius 8px
  box-shadow 0 4px 20px rgba(0, 0, 0, 0.15)
  z-index 1000
  min-width 160px
  padding 8px 0
  border 1px solid #e0e0e0

.context-menu-item
  padding 10px 16px
  cursor pointer
  display flex
  align-items center
  gap 10px
  font-size 14px
  color #333
  transition background 0.2s ease

  &:hover
    background #f5f5f5

.context-menu-icon
  width 18px
  height 18px
  color #666

// Hidden IDEs section
.hidden-ides-section
  margin-top 30px
  padding 20px
  background #f8f9fa
  border-radius 12px
  border 2px dashed #dee2e6

.hidden-ides-header
  display flex
  align-items center
  gap 10px
  color #6c757d
  font-size 14px
  font-weight 600
  margin-bottom 15px

.hidden-ides-icon
  width 20px
  height 20px

.hidden-ides-list
  display flex
  flex-wrap wrap
  gap 10px

.hidden-ide-btn
  background white
  border 2px solid var(--ide-color)
  color var(--ide-color)
  padding 8px 16px
  border-radius 8px
  font-size 13px
  font-weight 600
  cursor pointer
  transition all 0.2s ease

  &:hover
    background var(--ide-color)
    color white

@media (prefers-color-scheme: dark)
  .recent-section
    background linear-gradient(135deg, #4a5568 0%, #2d3748 100%)

  .recent-project-card
    background #2f2f2f

  .recent-project-header
    border-bottom-color #444

  .recent-project-name
    color #f6f6f6

  .recent-project-path
    color #aaa

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

  .context-menu
    background #2f2f2f
    border-color #444

    .context-menu-item
      color #f6f6f6

      &:hover
        background #3a3a3a

    .context-menu-icon
      color #aaa

  .hidden-ides-section
    background #2a2a2a
    border-color #444

  .hidden-ide-btn
    background #2f2f2f

    &:hover
      background var(--ide-color)
</style>
