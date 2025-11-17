# curl https://raw.githubusercontent.com/rcaloras/bash-preexec/master/bash-preexec.sh -o ~/.bash-preexec.sh
source ~/.bash-preexec.sh

LAST_OUTPUT=""
_OUTPUT_FILE="/tmp/bash_last_output_$$"

# Список интерактивных программ
_INTERACTIVE_REGEX="^(nano|vim|vi|emacs|mc|man|less|more|top|htop|ssh|tmux|screen|fzf)$"

# Перед выполнением команды
preexec() {
    local cmd_name=$(echo "$1" | awk '{print $1}' | xargs basename 2>/dev/null)

    # Если интерактивная программа - не перехватываем
    if [[ "$cmd_name" =~ $_INTERACTIVE_REGEX ]]; then
        _SKIP_CAPTURE=1
        return
    fi

    _SKIP_CAPTURE=0
    # Перенаправляем вывод в файл через exec
    exec > >(tee "$_OUTPUT_FILE")
    exec 2>&1
}

# После выполнения команды
precmd() {
    # Восстанавливаем stdout/stderr
    exec 1>&-
    exec 2>&-
    exec 1>/dev/tty
    exec 2>&1

    # Сохраняем вывод в переменную
    if [[ $_SKIP_CAPTURE -eq 0 ]] && [[ -f "$_OUTPUT_FILE" ]]; then
        LAST_OUTPUT=$(cat "$_OUTPUT_FILE")
        > "$_OUTPUT_FILE"  # Очищаем файл
    fi
}
