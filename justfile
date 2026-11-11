# Build both CLI tools in release mode.
build:
    cargo build --release --workspace

# Remove compiled artifacts.
clean:
    cargo clean

# Install both tools into INSTALL_DIR (defaults to ~/.local/bin).
# Existing executables are reported and require confirmation before replacement.
install: build
    #!/usr/bin/env bash
    install_dir="${INSTALL_DIR:-$HOME/.local/bin}"
    mkdir -p "$install_dir"
    existing=()
    for tool in zstd-seekify archive-mount; do
        if [ -e "$install_dir/$tool" ]; then
            existing+=("$install_dir/$tool")
        fi
    done
    if ((${#existing[@]})); then
        printf 'Warning: these files already exist and will be overwritten:\n'
        printf '  %s\n' "${existing[@]}"
        read -r -p 'Continue? [y/N] ' answer
        case "$answer" in
            y|Y|yes|YES) ;;
            *) echo 'Installation cancelled.'; exit 1 ;;
        esac
    fi
    install -m 755 target/release/zstd-seekify target/release/archive-mount "$install_dir/"
    echo "Installed zstd-seekify and archive-mount to $install_dir"

# Install shell completions for the detected shell, or COMPLETION_SHELL.
install-completions: build
    #!/usr/bin/env bash
    shell="${COMPLETION_SHELL:-${SHELL##*/}}"
    case "$shell" in
        bash)
            completion_dir="${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions"
            zstd_completion="$completion_dir/zstd-seekify"
            mount_completion="$completion_dir/archive-mount"
            ;;
        fish)
            completion_dir="${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"
            zstd_completion="$completion_dir/zstd-seekify.fish"
            mount_completion="$completion_dir/archive-mount.fish"
            ;;
        zsh)
            completion_dir="${ZSH_COMPLETION_DIR:-$HOME/.zfunc}"
            zstd_completion="$completion_dir/_zstd-seekify"
            mount_completion="$completion_dir/_archive-mount"
            ;;
        *)
            echo "Unsupported shell '$shell'. Set COMPLETION_SHELL to bash, fish, or zsh." >&2
            exit 1
            ;;
    esac
    mkdir -p "$completion_dir"
    existing=()
    for path in "$zstd_completion" "$mount_completion"; do
        if [ -e "$path" ]; then existing+=("$path"); fi
    done
    if ((${#existing[@]})); then
        printf 'Warning: these completion files already exist and will be overwritten:\n'
        printf '  %s\n' "${existing[@]}"
        read -r -p 'Continue? [y/N] ' answer
        case "$answer" in
            y|Y|yes|YES) ;;
            *) echo 'Completion installation cancelled.'; exit 1 ;;
        esac
    fi
    target/release/zstd-seekify --generate-completion "$shell" > "$zstd_completion"
    target/release/archive-mount --generate-completion "$shell" > "$mount_completion"
    echo "Installed $shell completions to $completion_dir"
    if [ "$shell" = zsh ]; then
        echo 'For zsh, ensure this directory is on fpath (for example: fpath=(~/.zfunc $fpath)).'
    fi

# Remove the installed tools from INSTALL_DIR.
uninstall:
    rm -f "${INSTALL_DIR:-$HOME/.local/bin}/zstd-seekify" "${INSTALL_DIR:-$HOME/.local/bin}/archive-mount"
