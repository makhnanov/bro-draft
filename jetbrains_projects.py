#!/usr/bin/env python3
"""
Скрипт для извлечения информации о недавних проектах из всех JetBrains IDE
"""

import os
import xml.etree.ElementTree as ET
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Optional
import json


class JetBrainsProject:
    def __init__(self, ide_name: str, ide_version: str, project_path: str, data: Dict):
        self.ide_name = ide_name
        self.ide_version = ide_version
        self.project_path = project_path
        self.display_name = data.get('displayName', '')
        self.frame_title = data.get('frameTitle', '')
        self.activation_timestamp = data.get('activationTimestamp')
        self.project_open_timestamp = data.get('projectOpenTimestamp')
        self.build = data.get('build', '')
        self.metadata = data.get('metadata', '')
        self.frame = data.get('frame', '')
        self.color_index = data.get('colorIndex')

    def get_activation_time(self) -> Optional[datetime]:
        if self.activation_timestamp:
            return datetime.fromtimestamp(int(self.activation_timestamp) / 1000)
        return None

    def get_open_time(self) -> Optional[datetime]:
        if self.project_open_timestamp:
            return datetime.fromtimestamp(int(self.project_open_timestamp) / 1000)
        return None

    def __repr__(self):
        activation = self.get_activation_time()
        activation_str = activation.strftime('%Y-%m-%d %H:%M:%S') if activation else 'N/A'

        name = self.display_name or Path(self.project_path).stem
        return (f"{self.ide_name} {self.ide_version} | {name}\n"
                f"  Path: {self.project_path}\n"
                f"  Last opened: {activation_str}\n"
                f"  Frame title: {self.frame_title}")


def parse_recent_projects_xml(file_path: Path, ide_name: str, ide_version: str) -> List[JetBrainsProject]:
    """Парсит XML файл с недавними проектами"""
    projects = []

    try:
        tree = ET.parse(file_path)
        root = tree.getroot()

        # Ищем component с недавними проектами
        for component in root.findall('.//component'):
            name_attr = component.get('name', '')
            if 'RecentProject' in name_attr or 'RecentSolution' in name_attr:
                # Парсим additionalInfo/map
                map_elem = component.find('.//option[@name="additionalInfo"]/map')
                if map_elem is not None:
                    for entry in map_elem.findall('entry'):
                        project_path = entry.get('key', '')

                        # Разворачиваем переменные окружения
                        project_path = project_path.replace('$USER_HOME$', str(Path.home()))
                        project_path = project_path.replace('$HOME$', str(Path.home()))

                        # Извлекаем метаданные проекта
                        project_data = {}
                        meta_info = entry.find('.//RecentProjectMetaInfo')
                        if meta_info is not None:
                            project_data['displayName'] = meta_info.get('displayName', '')
                            project_data['frameTitle'] = meta_info.get('frameTitle', '')

                            for option in meta_info.findall('option'):
                                name = option.get('name')
                                value = option.get('value')
                                if name and value:
                                    project_data[name] = value

                            # Извлекаем информацию о цвете
                            color_info = meta_info.find('.//RecentProjectColorInfo')
                            if color_info is not None:
                                project_data['colorIndex'] = color_info.get('associatedIndex')

                            # Извлекаем информацию о фрейме
                            frame = meta_info.find('frame')
                            if frame is not None:
                                project_data['frame'] = (
                                    f"x={frame.get('x')} y={frame.get('y')} "
                                    f"w={frame.get('width')} h={frame.get('height')}"
                                )

                        projects.append(JetBrainsProject(ide_name, ide_version, project_path, project_data))

                # Получаем последний открытый проект
                last_project_option = component.find('.//option[@name="lastOpenedProject"]')
                if last_project_option is not None:
                    last_project_path = last_project_option.get('value', '')
                    # Сохраняем информацию о последнем проекте
                    for project in projects:
                        if project.project_path == last_project_path:
                            project.is_last_opened = True

    except Exception as e:
        print(f"Error parsing {file_path}: {e}")

    return projects


def find_all_recent_projects() -> List[JetBrainsProject]:
    """Находит все недавние проекты во всех установленных JetBrains IDE"""
    all_projects = []

    config_dir = Path.home() / '.config' / 'JetBrains'

    if not config_dir.exists():
        print(f"Config directory not found: {config_dir}")
        return all_projects

    # Проходим по всем папкам IDE
    for ide_dir in config_dir.iterdir():
        if ide_dir.is_dir():
            # Извлекаем название IDE и версию
            ide_full_name = ide_dir.name

            # Разделяем название и версию (например, "Rider2025.1" -> "Rider", "2025.1")
            import re
            match = re.match(r'([A-Za-z]+)(\d+\.\d+(?:\.\d+)?)', ide_full_name)
            if match:
                ide_name = match.group(1)
                ide_version = match.group(2)
            else:
                ide_name = ide_full_name
                ide_version = 'Unknown'

            # Ищем файлы с недавними проектами
            options_dir = ide_dir / 'options'
            if options_dir.exists():
                for xml_file in ['recentProjects.xml', 'recentSolutions.xml']:
                    xml_path = options_dir / xml_file
                    if xml_path.exists():
                        projects = parse_recent_projects_xml(xml_path, ide_name, ide_version)
                        all_projects.extend(projects)

    return all_projects


def display_projects_by_ide(projects: List[JetBrainsProject]):
    """Отображает проекты, сгруппированные по IDE"""
    # Группируем проекты по IDE
    projects_by_ide = {}
    for project in projects:
        key = f"{project.ide_name} {project.ide_version}"
        if key not in projects_by_ide:
            projects_by_ide[key] = []
        projects_by_ide[key].append(project)

    # Сортируем проекты внутри каждой IDE по времени активации
    for ide_key in projects_by_ide:
        projects_by_ide[ide_key].sort(
            key=lambda p: p.activation_timestamp or '0',
            reverse=True
        )

    # Выводим информацию
    print("=" * 80)
    print("JetBrains IDE Recent Projects")
    print("=" * 80)
    print()

    for ide_key in sorted(projects_by_ide.keys()):
        print(f"\n{'=' * 80}")
        print(f"  {ide_key}")
        print('=' * 80)

        for i, project in enumerate(projects_by_ide[ide_key], 1):
            print(f"\n[{i}] {project.display_name or Path(project.project_path).stem}")
            print(f"    Path: {project.project_path}")

            if project.frame_title:
                print(f"    Frame: {project.frame_title}")

            activation = project.get_activation_time()
            if activation:
                print(f"    Last activated: {activation.strftime('%Y-%m-%d %H:%M:%S')}")

            open_time = project.get_open_time()
            if open_time:
                print(f"    Opened at: {open_time.strftime('%Y-%m-%d %H:%M:%S')}")

            if project.build:
                print(f"    Build: {project.build}")

            if project.frame:
                print(f"    Window: {project.frame}")

            if project.color_index is not None:
                print(f"    Color index: {project.color_index}")

            # Проверяем существование проекта
            if os.path.exists(project.project_path):
                print(f"    Status: ✓ Exists")
            else:
                print(f"    Status: ✗ Not found")

    print(f"\n{'=' * 80}")
    print(f"Total projects found: {len(projects)}")
    print('=' * 80)


def display_all_projects_sorted(projects: List[JetBrainsProject]):
    """Отображает все проекты, отсортированные по времени активации"""
    print("\n" + "=" * 80)
    print("All Projects (sorted by last activation)")
    print("=" * 80)

    sorted_projects = sorted(
        projects,
        key=lambda p: p.activation_timestamp or '0',
        reverse=True
    )

    for i, project in enumerate(sorted_projects, 1):
        activation = project.get_activation_time()
        activation_str = activation.strftime('%Y-%m-%d %H:%M:%S') if activation else 'N/A'

        name = project.display_name or Path(project.project_path).stem
        exists = "✓" if os.path.exists(project.project_path) else "✗"

        print(f"\n[{i}] {exists} {name}")
        print(f"    IDE: {project.ide_name} {project.ide_version}")
        print(f"    Path: {project.project_path}")
        print(f"    Last activated: {activation_str}")
        if project.frame_title:
            print(f"    Frame: {project.frame_title}")


def export_to_json(projects: List[JetBrainsProject], output_file: str = None):
    """Экспортирует проекты в JSON файл"""
    if output_file is None:
        output_file = os.environ.get('JSON_OUTPUT_PATH', 'jetbrains_projects.json')

    data = []
    for project in projects:
        data.append({
            'ide_name': project.ide_name,
            'ide_version': project.ide_version,
            'project_path': project.project_path,
            'display_name': project.display_name,
            'frame_title': project.frame_title,
            'activation_timestamp': project.activation_timestamp,
            'project_open_timestamp': project.project_open_timestamp,
            'activation_time': project.get_activation_time().isoformat() if project.get_activation_time() else None,
            'build': project.build,
            'frame': project.frame,
            'color_index': project.color_index,
            'exists': os.path.exists(project.project_path)
        })

    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)

    print(f"\nExported to {output_file}")


def main():
    # Если запущен из Rust, только экспортируем JSON без вывода
    if 'JSON_OUTPUT_PATH' in os.environ:
        projects = find_all_recent_projects()
        export_to_json(projects)
        return

    # Иначе показываем полный вывод
    print("Scanning JetBrains IDE configurations...")
    projects = find_all_recent_projects()

    if not projects:
        print("No projects found.")
        return

    # Отображаем проекты по IDE
    display_projects_by_ide(projects)

    # Отображаем все проекты, отсортированные по времени
    display_all_projects_sorted(projects)

    # Экспортируем в JSON
    export_to_json(projects)


if __name__ == '__main__':
    main()
