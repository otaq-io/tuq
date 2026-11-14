#!/bin/sh
# tuq's installer for macOS and Linux. Installs or updates tuq on one agent tool, and the tuq and tuq-plan commands:
#
#   curl -fsSL https://raw.githubusercontent.com/otaq-io/tuq/main/install.sh | sh -s -- --tool claude
#
# --tool claude|codex|copilot|opencode   the tool to install tuq on (required). Run it once per tool you use.
#
# What it does, each step safe to run again (running it again is how tuq is updated):
#  1. Downloads the newest release's commands for this system (tuq-macos-universal.tar.gz, tuq-linux-x64.tar.gz or
#     tuq-linux-arm64.tar.gz), checks them against the release's SHA256SUMS and puts tuq and tuq-plan in ~/.tuq/bin.
#  2. Puts ~/.tuq/bin on the PATH once, in the shell's startup file (~/.zshrc; ~/.bashrc and ~/.bash_profile or
#     ~/.profile; fish's conf.d; else ~/.profile). New terminals see it.
#  3. Installs or updates tuq the tool's own way, from the otaq-io/tuq repository:
#       Claude Code  claude plugin marketplace add otaq-io/tuq   (or: claude plugin marketplace update otaq-claude-skills)
#                    claude plugin install tuq@otaq-claude-skills --scope user   (or: claude plugin update ...)
#       Codex        codex plugin marketplace add otaq-io/tuq    (or: codex plugin marketplace upgrade otaq-claude-skills)
#                    codex plugin add tuq@otaq-claude-skills
#       Copilot      copilot plugin marketplace add otaq-io/tuq  (or: copilot plugin marketplace update otaq-claude-skills)
#                    copilot plugin install tuq@otaq-claude-skills   (or: copilot plugin update ...)
#       OpenCode     copies plugins/tuq-prefixed/skills/tuq-* into ${XDG_CONFIG_HOME:-~/.config}/opencode/skills
#     Claude Code and Codex read .claude-plugin/marketplace.json (skills named tuq:<name>); Copilot reads
#     .github/plugin/marketplace.json, whose tuq is the prefixed plugin (skills named tuq-<name>), as OpenCode's are.
#  4. Prints the installed version. When something stops it, it says why and exits non-zero.
#
# For testing a release before it is published:
#   --from <folder>    take the release assets (the archives and SHA256SUMS) from this folder, not GitHub
#   --source <where>   take the skills from here, not otaq-io/tuq: a checkout's folder, owner/repo or a git URL
#
# Needs: curl, tar, and the tool itself (claude, codex or copilot on the PATH). Not for Windows: use install.ps1.

set -eu

REPO="otaq-io/tuq"
MARKETPLACE="otaq-claude-skills"
PLUGIN="tuq@otaq-claude-skills"

say() {
    printf '%s\n' "$*"
}

fail() {
    printf 'tuq install: %s\n' "$*" >&2
    exit 1
}

usage() {
    say "usage: install.sh --tool claude|codex|copilot|opencode [--from <folder>] [--source <folder|owner/repo|git URL>]"
}

# run <command...>: shows and runs a command, its input never the piped script; stops with its output on failure.
run() {
    say "  \$ $*"
    if ! out="$("$@" </dev/null 2>&1)"; then
        printf '%s\n' "$out" | sed 's/^/    /' >&2
        fail "'$*' failed"
    fi
    [ -z "$out" ] || printf '%s\n' "$out" | sed 's/^/    /'
}

need() {
    command -v "$1" >/dev/null 2>&1 || fail "$1 isn't installed or isn't on the PATH ($2)"
}

# --- 1. the commands ----------------------------------------------------------------------------------------------

asset_name() {
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os" in
    Darwin) echo "tuq-macos-universal.tar.gz" ;;
    Linux)
        case "$arch" in
        x86_64 | amd64) echo "tuq-linux-x64.tar.gz" ;;
        aarch64 | arm64) echo "tuq-linux-arm64.tar.gz" ;;
        *) fail "there is no tuq release for Linux on $arch (only x86_64 and arm64)" ;;
        esac
        ;;
    MINGW* | MSYS* | CYGWIN*) fail "on Windows, run install.ps1 in PowerShell instead" ;;
    *) fail "there is no tuq release for $os" ;;
    esac
}

sha256_of() {
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d' ' -f1
    elif command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    else
        fail "neither shasum nor sha256sum is installed, so the download can't be checked"
    fi
}

# fetch <name> <to>: one release asset, from --from's folder or the newest GitHub release.
fetch() {
    if [ -n "$from" ]; then
        [ -f "$from/$1" ] || fail "$from has no $1"
        cp "$from/$1" "$2"
    else
        curl -fsSL --retry 2 -o "$2" "https://github.com/$release_repo/releases/latest/download/$1" ||
            fail "couldn't download $1 from https://github.com/$release_repo/releases/latest (offline, or no release yet?)"
    fi
}

install_commands() {
    asset="$(asset_name)"
    say "Downloading tuq's commands ($asset)"
    fetch SHA256SUMS "$tmp/SHA256SUMS"
    fetch "$asset" "$tmp/$asset"
    expected="$(awk -v n="$asset" '$2 == n || $2 == "*" n { print $1 }' "$tmp/SHA256SUMS")"
    [ -n "$expected" ] || fail "SHA256SUMS has no line for $asset"
    actual="$(sha256_of "$tmp/$asset")"
    [ "$expected" = "$actual" ] || fail "$asset doesn't match SHA256SUMS (expected $expected, got $actual); nothing was installed"
    mkdir -p "$tmp/unpacked"
    tar -xzf "$tmp/$asset" -C "$tmp/unpacked" || fail "couldn't unpack $asset"
    mkdir -p "$bin_dir" || fail "couldn't create $bin_dir"
    for program in tuq tuq-plan; do
        [ -f "$tmp/unpacked/$program" ] || fail "$asset has no $program"
        chmod 755 "$tmp/unpacked/$program"
        # Copied next to it first, then renamed over the old one: a running tuq keeps its file.
        cp "$tmp/unpacked/$program" "$bin_dir/.$program.new" && mv -f "$bin_dir/.$program.new" "$bin_dir/$program" ||
            fail "couldn't write $bin_dir/$program"
    done
    commands_version="$("$bin_dir/tuq" --version 2>/dev/null)" || fail "$bin_dir/tuq doesn't run on this system"
    commands_version="${commands_version#tuq }"
    say "  tuq and tuq-plan $commands_version are in $bin_dir"
}

# --- 2. the PATH ----------------------------------------------------------------------------------------------------

# add_path_line <file> <line>: appends the line once; a file that already names ~/.tuq/bin is left alone.
add_path_line() {
    if [ -f "$1" ] && grep -q '\.tuq/bin' "$1"; then
        return
    fi
    mkdir -p "$(dirname "$1")"
    printf '\n# tuq'"'"'s commands (added by tuq'"'"'s installer)\n%s\n' "$2" >>"$1" || fail "couldn't write $1"
    say "  added ~/.tuq/bin to the PATH in $1"
    path_changed=true
}

put_on_path() {
    say "Putting ~/.tuq/bin on the PATH"
    path_changed=false
    sh_line='export PATH="$HOME/.tuq/bin:$PATH"'
    case "$(basename "${SHELL:-sh}")" in
    zsh) add_path_line "${ZDOTDIR:-$HOME}/.zshrc" "$sh_line" ;;
    bash)
        add_path_line "$HOME/.bashrc" "$sh_line"
        # A login bash reads the first of these it finds; creating .bash_profile would hide the user's .profile.
        if [ -f "$HOME/.bash_profile" ]; then
            add_path_line "$HOME/.bash_profile" "$sh_line"
        elif [ -f "$HOME/.bash_login" ]; then
            add_path_line "$HOME/.bash_login" "$sh_line"
        else
            add_path_line "$HOME/.profile" "$sh_line"
        fi
        ;;
    fish) add_path_line "${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d/tuq.fish" 'fish_add_path -g "$HOME/.tuq/bin"' ;;
    *) add_path_line "$HOME/.profile" "$sh_line" ;;
    esac
    $path_changed || say "  already there"
}

# --- 3. the tool ----------------------------------------------------------------------------------------------------

install_claude() {
    need claude "https://claude.com/claude-code"
    if claude plugin marketplace list --json </dev/null 2>/dev/null | grep -q "\"name\": *\"$MARKETPLACE\""; then
        run claude plugin marketplace update "$MARKETPLACE"
    else
        run claude plugin marketplace add "$source"
    fi
    if claude plugin list --json </dev/null 2>/dev/null | grep -q "\"id\": *\"$PLUGIN\""; then
        run claude plugin update "$PLUGIN"
    else
        run claude plugin install "$PLUGIN" --scope user
    fi
}

install_codex() {
    need codex "https://developers.openai.com/codex/cli"
    if codex plugin marketplace list </dev/null 2>/dev/null | grep -q "^$MARKETPLACE "; then
        # Only a git marketplace has anything to refresh; a folder is read as it is.
        say "  \$ codex plugin marketplace upgrade $MARKETPLACE"
        if ! out="$(codex plugin marketplace upgrade "$MARKETPLACE" </dev/null 2>&1)"; then
            case "$out" in
            *"not configured as a Git marketplace"*) ;;
            *)
                printf '%s\n' "$out" | sed 's/^/    /' >&2
                fail "'codex plugin marketplace upgrade $MARKETPLACE' failed"
                ;;
            esac
        fi
    else
        run codex plugin marketplace add "$source"
    fi
    # Adding again copies the marketplace's current tuq over the installed one.
    run codex plugin add "$PLUGIN"
}

install_copilot() {
    need copilot "https://github.com/features/copilot/cli"
    if copilot plugin marketplace list </dev/null 2>/dev/null | grep -q "$MARKETPLACE"; then
        run copilot plugin marketplace update "$MARKETPLACE"
    else
        run copilot plugin marketplace add "$source"
    fi
    if copilot plugin list </dev/null 2>/dev/null | grep -q "$PLUGIN"; then
        run copilot plugin update "$PLUGIN"
    else
        run copilot plugin install "$PLUGIN"
    fi
}

# The prefixed skills' folder of the source: the folder itself, or a download or clone of it into $tmp.
prefixed_skills() {
    if [ -d "$source" ]; then
        dir="$source/plugins/tuq-prefixed/skills"
    elif printf '%s' "$source" | grep -Eq '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$'; then
        curl -fsSL --retry 2 -o "$tmp/source.tar.gz" "https://github.com/$source/archive/HEAD.tar.gz" ||
            fail "couldn't download https://github.com/$source (offline?)"
        mkdir -p "$tmp/source"
        tar -xzf "$tmp/source.tar.gz" -C "$tmp/source" || fail "couldn't unpack $source's files"
        dir="$(find "$tmp/source" -mindepth 4 -maxdepth 4 -type d -path '*/plugins/tuq-prefixed/skills' | head -n 1)"
    else
        need git "to clone $source"
        git clone --quiet --depth 1 "$source" "$tmp/source" </dev/null >/dev/null 2>&1 || fail "couldn't clone $source"
        dir="$tmp/source/plugins/tuq-prefixed/skills"
    fi
    [ -n "$dir" ] && [ -d "$dir" ] || fail "$source has no plugins/tuq-prefixed/skills"
    echo "$dir"
}

install_opencode() {
    command -v opencode >/dev/null 2>&1 ||
        say "  (OpenCode isn't on the PATH; tuq's skills go where it will find them: https://opencode.ai)"
    from_dir="$(prefixed_skills)"
    skills_dir="${XDG_CONFIG_HOME:-$HOME/.config}/opencode/skills"
    mkdir -p "$skills_dir" || fail "couldn't create $skills_dir"
    say "  copying tuq's skills into $skills_dir"
    names=""
    for skill in "$from_dir"/tuq-*/; do
        name="$(basename "$skill")"
        [ -f "$skill/SKILL.md" ] || continue
        names="$names $name "
        rm -rf "$skills_dir/.$name.new"
        cp -R "$skill" "$skills_dir/.$name.new" || fail "couldn't copy $name into $skills_dir"
        rm -rf "$skills_dir/$name" && mv "$skills_dir/.$name.new" "$skills_dir/$name" ||
            fail "couldn't replace $skills_dir/$name"
    done
    [ -n "$names" ] || fail "$from_dir has no tuq-* skills"
    # A skill an earlier tuq release installed and this one no longer has (only folders carrying tuq's stamp).
    for old in "$skills_dir"/tuq-*/; do
        [ -d "$old" ] || continue
        name="$(basename "$old")"
        case "$names" in *" $name "*) continue ;; esac
        if grep -q '^  tuq-release:' "$old/SKILL.md" 2>/dev/null; then
            rm -rf "$old"
            say "  removed $name, which this tuq no longer has"
        fi
    done
}

# --- 4. the version -------------------------------------------------------------------------------------------------

# The installed skills' metadata.tuq-release (empty when unknown): read from the copy the tool loads.
skills_version() {
    file=""
    case "$tool" in
    claude)
        # The plugin's folder: readFromFolder (a folder marketplace) or installPath.
        folder="$(claude plugin list --json </dev/null 2>/dev/null |
            awk -v id="\"$PLUGIN\"" '
                index($0, id) { found = 1 }
                found && /"readFromFolder"/ { sub(/.*"readFromFolder": *"/, ""); sub(/".*/, ""); rf = $0 }
                found && /"installPath"/ { sub(/.*"installPath": *"/, ""); sub(/".*/, ""); ip = $0 }
                found && /^ *}/ { exit }
                END { print (rf != "" ? rf : ip) }')"
        file="$folder/skills/product-spec/SKILL.md"
        ;;
    codex)
        file="$(ls -t "${CODEX_HOME:-$HOME/.codex}"/plugins/cache/"$MARKETPLACE"/tuq/*/skills/product-spec/SKILL.md 2>/dev/null | head -n 1)"
        ;;
    copilot)
        file="$(find "${COPILOT_HOME:-$HOME/.copilot}/installed-plugins" -path '*/skills/tuq-product-spec/SKILL.md' 2>/dev/null | head -n 1)"
        # A folder marketplace is loaded from the folder, never copied.
        [ -n "$file" ] || { [ -d "$source" ] && file="$source/plugins/tuq-prefixed/skills/tuq-product-spec/SKILL.md"; }
        ;;
    opencode) file="${XDG_CONFIG_HOME:-$HOME/.config}/opencode/skills/tuq-product-spec/SKILL.md" ;;
    esac
    [ -n "$file" ] && [ -f "$file" ] || return 0
    sed -n 's/^  tuq-release: *//p' "$file" | head -n 1
}

# --- main -----------------------------------------------------------------------------------------------------------

main() {
    tool=""
    from=""
    source=""
    while [ $# -gt 0 ]; do
        case "$1" in
        --tool) [ $# -ge 2 ] || fail "--tool needs a value"; tool="$2"; shift 2 ;;
        --tool=*) tool="${1#--tool=}"; shift ;;
        --from) [ $# -ge 2 ] || fail "--from needs a folder"; from="$2"; shift 2 ;;
        --from=*) from="${1#--from=}"; shift ;;
        --source) [ $# -ge 2 ] || fail "--source needs a value"; source="$2"; shift 2 ;;
        --source=*) source="${1#--source=}"; shift ;;
        -h | --help) usage; exit 0 ;;
        *) usage >&2; fail "unknown argument: $1" ;;
        esac
    done
    case "$tool" in
    claude) tool_name="Claude Code" ;;
    codex) tool_name="Codex" ;;
    copilot) tool_name="GitHub Copilot" ;;
    opencode) tool_name="OpenCode" ;;
    "") usage >&2; fail "say which tool: --tool claude|codex|copilot|opencode" ;;
    *) fail "unknown tool '$tool': use claude, codex, copilot or opencode" ;;
    esac
    [ -z "$from" ] || [ -d "$from" ] || fail "--from $from isn't a folder"
    [ -n "$from" ] || need curl "to download tuq"
    need tar "to unpack tuq"
    [ -n "$source" ] || source="$REPO"
    case "$source" in
    /* | ./* | ../*) [ -d "$source" ] || fail "--source $source isn't a folder" ;;
    esac
    [ ! -d "$source" ] || source="$(cd "$source" && pwd)"
    # The commands come from the releases of the repository the skills come from, when that is a GitHub one.
    release_repo="$REPO"
    if printf '%s' "$source" | grep -Eq '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$' && [ ! -d "$source" ]; then
        release_repo="$source"
    fi
    [ -n "${HOME:-}" ] || fail "HOME isn't set"
    bin_dir="$HOME/.tuq/bin"

    tmp="$(mktemp -d 2>/dev/null || mktemp -d -t tuq)"
    trap 'rm -rf "$tmp"' EXIT INT TERM

    say "Installing tuq for $tool_name"
    install_commands
    put_on_path
    say "Installing tuq's skills in $tool_name"
    "install_$tool"

    version="$(skills_version)"
    say ""
    if [ -n "$version" ]; then
        say "tuq $version is installed for $tool_name (commands $commands_version in $bin_dir)."
    else
        say "tuq is installed for $tool_name (commands $commands_version in $bin_dir)."
    fi
    case "$tool" in
    claude | codex) say "Its skills are tuq:<name>: /tuq:product-spec in Claude Code, \$tuq:product-spec in Codex." ;;
    copilot | opencode) say "Its skills are tuq-<name> there: /tuq-product-spec." ;;
    esac
    case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) say "Open a new terminal (or run: export PATH=\"\$HOME/.tuq/bin:\$PATH\") to use tuq and tuq-plan." ;;
    esac
    say "Run this again to update tuq."
}

main "$@"
