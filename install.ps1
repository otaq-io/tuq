# tuq's installer for Windows (Windows PowerShell 5.1 or PowerShell 7; no bash). Installs or updates tuq on one agent
# tool, and the tuq and tuq-plan commands:
#
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/otaq-io/tuq/main/install.ps1))) --tool claude
#   powershell -ExecutionPolicy Bypass -File install.ps1 --tool claude          (a downloaded copy)
#
# --tool claude|codex|copilot|opencode   the tool to install tuq on (required). Run it once per tool you use.
#
# What it does, each step safe to run again (running it again is how tuq is updated):
#  1. Downloads the newest release's commands for this system (tuq-windows-x64.zip or tuq-windows-arm64.zip), checks
#     them against the release's SHA256SUMS and puts tuq.exe and tuq-plan.exe in %LOCALAPPDATA%\tuq\bin.
#  2. Adds that folder to the user's Path once (HKCU\Environment, as System Properties does). New terminals see it.
#  3. Installs or updates tuq the tool's own way, from the otaq-io/tuq repository, with the same commands as
#     install.sh (see its header): claude/codex/copilot plugin marketplace add otaq-io/tuq, then install tuq@otaq-claude-skills;
#     on OpenCode, copies plugins/tuq-prefixed/skills/tuq-* into %XDG_CONFIG_HOME%\opencode\skills, else
#     %USERPROFILE%\.config\opencode\skills.
#  4. Prints the installed version. When something stops it, it says why and fails (exit code 1 when run as a file).
#
# For testing a release before it is published:
#   --from <folder>    take the release assets (the archives and SHA256SUMS) from this folder, not GitHub
#   --source <where>   take the skills from here, not otaq-io/tuq: a checkout's folder, owner/repo or a git URL

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Repo = 'otaq-io/tuq'
$Marketplace = 'otaq-claude-skills'
$Plugin = 'tuq@otaq-claude-skills'
$RunAsFile = [bool]$PSCommandPath

# A reason the install stops, told apart from PowerShell's own errors by its Data['tuq'] mark.
function Fail([string]$Message) {
    $e = [System.Exception]::new($Message)
    $e.Data['tuq'] = $true
    throw $e
}

function Say([string]$Message) {
    Write-Host $Message
}

function Usage {
    'usage: install.ps1 --tool claude|codex|copilot|opencode [--from <folder>] [--source <folder|owner/repo|git URL>]'
}

# The tool's program: an .exe or .cmd, never npm's .ps1 shim (the execution policy often blocks those) or its
# extensionless sh shim.
function Find-Tool([string]$Name) {
    $found = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue |
        Where-Object { $_.Extension -in '.exe', '.cmd', '.bat', '.com' } | Select-Object -First 1
    if ($found) { return $found.Source }
    return $null
}

function Need-Tool([string]$Name, [string]$Where) {
    $exe = Find-Tool $Name
    if (-not $exe) { Fail "$Name isn't installed or isn't on the PATH ($Where)" }
    return $exe
}

# Runs a program and returns its output (stdout and stderr) and exit code, without stopping on its stderr.
function Invoke-Quiet([string]$Exe, [string[]]$Arguments) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $out = & $Exe @Arguments 2>&1 | ForEach-Object { "$_" } | Out-String
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $saved
    }
    return [pscustomobject]@{ Output = $out.TrimEnd(); Code = $code }
}

function Show-Output([string]$Text) {
    if ($Text) { ($Text -split "`r?`n" | ForEach-Object { "    $_" }) -join "`n" | Write-Host }
}

# Shows and runs a command; stops with its output when it fails.
function Run([string]$Exe, [string[]]$Arguments, [string]$Shown) {
    Say "  > $Shown $($Arguments -join ' ')"
    $r = Invoke-Quiet $Exe $Arguments
    Show-Output $r.Output
    if ($r.Code -ne 0) { Fail "'$Shown $($Arguments -join ' ')' failed" }
}

# --- 1. the commands ----------------------------------------------------------------------------------------------

function Get-AssetName {
    # The system's own architecture, also when this PowerShell runs as x64 on an Arm PC.
    $arch = (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment' -ErrorAction SilentlyContinue).PROCESSOR_ARCHITECTURE
    if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
    switch ($arch) {
        'ARM64' { return 'tuq-windows-arm64.zip' }
        'AMD64' { return 'tuq-windows-x64.zip' }
        default { Fail "there is no tuq release for Windows on $arch (only x64 and arm64)" }
    }
}

function Get-Asset([string]$Name, [string]$To) {
    if ($script:From) {
        $path = Join-Path $script:From $Name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { Fail "$($script:From) has no $Name" }
        Copy-Item -LiteralPath $path $To
    } else {
        $url = "https://github.com/$($script:ReleaseRepo)/releases/latest/download/$Name"
        try {
            Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $To
        } catch {
            Fail "couldn't download $Name from https://github.com/$($script:ReleaseRepo)/releases/latest (offline, or no release yet?): $($_.Exception.Message)"
        }
    }
}

function Install-Commands {
    $asset = Get-AssetName
    Say "Downloading tuq's commands ($asset)"
    Get-Asset 'SHA256SUMS' (Join-Path $script:Tmp 'SHA256SUMS')
    Get-Asset $asset (Join-Path $script:Tmp $asset)
    $expected = $null
    foreach ($line in Get-Content -LiteralPath (Join-Path $script:Tmp 'SHA256SUMS')) {
        $parts = $line.Trim() -split '\s+', 2
        if ($parts.Count -eq 2 -and $parts[1].TrimStart('*') -eq $asset) { $expected = $parts[0].ToLowerInvariant() }
    }
    if (-not $expected) { Fail "SHA256SUMS has no line for $asset" }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $script:Tmp $asset)).Hash.ToLowerInvariant()
    if ($expected -ne $actual) { Fail "$asset doesn't match SHA256SUMS (expected $expected, got $actual); nothing was installed" }
    $unpacked = Join-Path $script:Tmp 'unpacked'
    try {
        Expand-Archive -LiteralPath (Join-Path $script:Tmp $asset) -DestinationPath $unpacked -Force
    } catch { Fail "couldn't unpack ${asset}: $($_.Exception.Message)" }
    New-Item -ItemType Directory -Force -Path $script:BinDir | Out-Null
    foreach ($program in 'tuq.exe', 'tuq-plan.exe') {
        $new = Join-Path $unpacked $program
        if (-not (Test-Path -LiteralPath $new -PathType Leaf)) { Fail "$asset has no $program" }
        $target = Join-Path $script:BinDir $program
        try {
            # A running program can be renamed but not overwritten: move the old one aside first.
            if (Test-Path -LiteralPath $target) {
                $old = "$target.old"
                Remove-Item -LiteralPath $old -Force -ErrorAction SilentlyContinue
                Move-Item -LiteralPath $target $old -Force
            }
            Copy-Item -LiteralPath $new $target -Force
            Remove-Item -LiteralPath "$target.old" -Force -ErrorAction SilentlyContinue
        } catch { Fail "couldn't write ${target}: $($_.Exception.Message)" }
    }
    $r = Invoke-Quiet (Join-Path $script:BinDir 'tuq.exe') @('--version')
    if ($r.Code -ne 0 -or $r.Output -notmatch '^tuq ') { Fail "$($script:BinDir)\tuq.exe doesn't run on this system: $($r.Output)" }
    $script:CommandsVersion = $r.Output.Trim() -replace '^tuq ', ''
    Say "  tuq.exe and tuq-plan.exe $($script:CommandsVersion) are in $($script:BinDir)"
}

# --- 2. the PATH ----------------------------------------------------------------------------------------------------

function Test-SamePath([string]$A, [string]$B) {
    $norm = { param($p) [Environment]::ExpandEnvironmentVariables($p).Trim().TrimEnd('\').ToLowerInvariant() }
    return (& $norm $A) -eq (& $norm $B)
}

function Send-SettingChange {
    if (-not ('TuqInstaller.Env' -as [type])) {
        Add-Type -Namespace TuqInstaller -Name Env -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
    }
    $result = [UIntPtr]::Zero
    # HWND_BROADCAST, WM_SETTINGCHANGE, "Environment", SMTO_ABORTIFHUNG: Explorer and new terminals read the new Path.
    [void][TuqInstaller.Env]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$result)
}

function Add-ToUserPath {
    Say 'Putting tuq on your Path'
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
    if (-not $key) { Fail "couldn't open HKCU\Environment" }
    try {
        # Read as stored (%VARS% unexpanded) and written back as the same kind, so no other entry changes.
        $current = $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $kind = if ($key.GetValueNames() -contains 'Path') { $key.GetValueKind('Path') } else { [Microsoft.Win32.RegistryValueKind]::ExpandString }
        $entries = @($current -split ';' | Where-Object { $_ -ne '' })
        if ($entries | Where-Object { Test-SamePath $_ $script:BinDir }) {
            Say '  already there'
        } else {
            $new = (@($entries) + $script:BinDir) -join ';'
            $key.SetValue('Path', $new, $kind)
            Send-SettingChange
            Say "  added $($script:BinDir) to your user Path"
        }
    } catch {
        if ($_.Exception.Data['tuq']) { throw }
        Fail "couldn't change your user Path: $($_.Exception.Message)"
    } finally {
        $key.Close()
    }
    if (-not (($env:Path -split ';') | Where-Object { $_ -and (Test-SamePath $_ $script:BinDir) })) {
        $env:Path = "$env:Path;$($script:BinDir)"
    }
}

# --- 3. the tool ----------------------------------------------------------------------------------------------------

function Install-Claude {
    $claude = Need-Tool 'claude' 'https://claude.com/claude-code'
    $marketplaces = (Invoke-Quiet $claude @('plugin', 'marketplace', 'list', '--json')).Output
    if ($marketplaces -match "`"name`":\s*`"$Marketplace`"") {
        Run $claude @('plugin', 'marketplace', 'update', $Marketplace) 'claude'
    } else {
        Run $claude @('plugin', 'marketplace', 'add', $script:Source) 'claude'
    }
    $plugins = (Invoke-Quiet $claude @('plugin', 'list', '--json')).Output
    if ($plugins -match "`"id`":\s*`"$Plugin`"") {
        Run $claude @('plugin', 'update', $Plugin) 'claude'
    } else {
        Run $claude @('plugin', 'install', $Plugin, '--scope', 'user') 'claude'
    }
}

function Install-Codex {
    $codex = Need-Tool 'codex' 'https://developers.openai.com/codex/cli'
    $marketplaces = (Invoke-Quiet $codex @('plugin', 'marketplace', 'list')).Output
    if ($marketplaces -match "(?m)^$Marketplace\s") {
        # Only a git marketplace has anything to refresh; a folder is read as it is.
        Say "  > codex plugin marketplace upgrade $Marketplace"
        $r = Invoke-Quiet $codex @('plugin', 'marketplace', 'upgrade', $Marketplace)
        if ($r.Code -ne 0 -and $r.Output -notmatch 'not configured as a Git marketplace') {
            Show-Output $r.Output
            Fail "'codex plugin marketplace upgrade $Marketplace' failed"
        }
    } else {
        Run $codex @('plugin', 'marketplace', 'add', $script:Source) 'codex'
    }
    # Adding again copies the marketplace's current tuq over the installed one.
    Run $codex @('plugin', 'add', $Plugin) 'codex'
}

function Install-Copilot {
    $copilot = Need-Tool 'copilot' 'https://github.com/features/copilot/cli'
    $marketplaces = (Invoke-Quiet $copilot @('plugin', 'marketplace', 'list')).Output
    if ($marketplaces -match [regex]::Escape($Marketplace)) {
        Run $copilot @('plugin', 'marketplace', 'update', $Marketplace) 'copilot'
    } else {
        Run $copilot @('plugin', 'marketplace', 'add', $script:Source) 'copilot'
    }
    # A marketplace's plugins are listed "(disabled)" until installed; installing also turns a disabled tuq back on.
    $listed = ((Invoke-Quiet $copilot @('plugin', 'list')).Output -split "`r?`n") | Where-Object { $_.Contains($Plugin) }
    if ($listed -and -not ($listed -match 'disabled')) {
        Run $copilot @('plugin', 'update', $Plugin) 'copilot'
    } else {
        Run $copilot @('plugin', 'install', $Plugin) 'copilot'
    }
}

function Test-GitHubRepo([string]$Value) {
    return ($Value -match '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$') -and -not (Test-Path -LiteralPath $Value)
}

# The prefixed skills' folder of the source: the folder itself, or a download or clone of it.
function Get-PrefixedSkills {
    if (Test-Path -LiteralPath $script:Source -PathType Container) {
        $dir = Join-Path $script:Source 'plugins\tuq-prefixed\skills'
    } elseif (Test-GitHubRepo $script:Source) {
        $zip = Join-Path $script:Tmp 'source.zip'
        try {
            Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/$($script:Source)/archive/HEAD.zip" -OutFile $zip
        } catch { Fail "couldn't download https://github.com/$($script:Source) (offline?): $($_.Exception.Message)" }
        $unpacked = Join-Path $script:Tmp 'source'
        Expand-Archive -LiteralPath $zip -DestinationPath $unpacked -Force
        $top = Get-ChildItem -LiteralPath $unpacked -Directory | Select-Object -First 1
        if (-not $top) { Fail "$($script:Source)'s download is empty" }
        $dir = Join-Path $top.FullName 'plugins\tuq-prefixed\skills'
    } else {
        $git = Need-Tool 'git' "to clone $($script:Source)"
        $clone = Join-Path $script:Tmp 'source'
        $r = Invoke-Quiet $git @('clone', '--quiet', '--depth', '1', $script:Source, $clone)
        if ($r.Code -ne 0) { Fail "couldn't clone $($script:Source): $($r.Output)" }
        $dir = Join-Path $clone 'plugins\tuq-prefixed\skills'
    }
    if (-not (Test-Path -LiteralPath $dir -PathType Container)) { Fail "$($script:Source) has no plugins/tuq-prefixed/skills" }
    return $dir
}

function Get-OpenCodeSkillsDir {
    $config = if ($env:XDG_CONFIG_HOME) { $env:XDG_CONFIG_HOME } else { Join-Path $env:USERPROFILE '.config' }
    return Join-Path $config 'opencode\skills'
}

function Install-OpenCode {
    if (-not (Find-Tool 'opencode')) { Say "  (OpenCode isn't on the PATH; tuq's skills go where it will find them: https://opencode.ai)" }
    $fromDir = Get-PrefixedSkills
    $skillsDir = Get-OpenCodeSkillsDir
    New-Item -ItemType Directory -Force -Path $skillsDir | Out-Null
    Say "  copying tuq's skills into $skillsDir"
    $names = @()
    foreach ($skill in Get-ChildItem -LiteralPath $fromDir -Directory -Filter 'tuq-*') {
        if (-not (Test-Path -LiteralPath (Join-Path $skill.FullName 'SKILL.md'))) { continue }
        $names += $skill.Name
        $staged = Join-Path $skillsDir ".$($skill.Name).new"
        $target = Join-Path $skillsDir $skill.Name
        try {
            if (Test-Path -LiteralPath $staged) { Remove-Item -LiteralPath $staged -Recurse -Force }
            Copy-Item -LiteralPath $skill.FullName $staged -Recurse
            if (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target -Recurse -Force }
            Rename-Item -LiteralPath $staged $skill.Name
        } catch { Fail "couldn't put $($skill.Name) in ${skillsDir}: $($_.Exception.Message)" }
    }
    if ($names.Count -eq 0) { Fail "$fromDir has no tuq-* skills" }
    # A skill an earlier tuq release installed and this one no longer has (only folders carrying tuq's stamp).
    foreach ($old in Get-ChildItem -LiteralPath $skillsDir -Directory -Filter 'tuq-*') {
        if ($names -contains $old.Name) { continue }
        $md = Join-Path $old.FullName 'SKILL.md'
        if ((Test-Path -LiteralPath $md) -and (Select-String -LiteralPath $md -Pattern '^  tuq-release:' -Quiet)) {
            Remove-Item -LiteralPath $old.FullName -Recurse -Force
            Say "  removed $($old.Name), which this tuq no longer has"
        }
    }
}

# --- 4. the version -------------------------------------------------------------------------------------------------

# The installed skills' metadata.tuq-release ($null when unknown): read from the copy the tool loads.
function Get-SkillsVersion {
    $file = $null
    switch ($script:Tool) {
        'claude' {
            $claude = Find-Tool 'claude'
            try {
                $list = (Invoke-Quiet $claude @('plugin', 'list', '--json')).Output | ConvertFrom-Json
                $entry = @($list) | Where-Object { $_.id -eq $Plugin } | Select-Object -First 1
                $folder = if ($entry.readFromFolder) { $entry.readFromFolder } else { $entry.installPath }
                if ($folder) { $file = Join-Path $folder 'skills\product-spec\SKILL.md' }
            } catch { }
        }
        'codex' {
            $home_ = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $env:USERPROFILE '.codex' }
            $file = Get-ChildItem -Path (Join-Path $home_ "plugins\cache\$Marketplace\tuq\*\skills\product-spec\SKILL.md") -ErrorAction SilentlyContinue |
                Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
        }
        'copilot' {
            $home_ = if ($env:COPILOT_HOME) { $env:COPILOT_HOME } else { Join-Path $env:USERPROFILE '.copilot' }
            $file = Get-ChildItem -Path (Join-Path $home_ 'installed-plugins') -Recurse -Filter 'SKILL.md' -ErrorAction SilentlyContinue |
                Where-Object { $_.Directory.Name -eq 'tuq-product-spec' } | Select-Object -First 1 -ExpandProperty FullName
            # A folder marketplace is loaded from the folder, never copied.
            if (-not $file -and (Test-Path -LiteralPath $script:Source -PathType Container)) {
                $file = Join-Path $script:Source 'plugins\tuq-prefixed\skills\tuq-product-spec\SKILL.md'
            }
        }
        'opencode' { $file = Join-Path (Get-OpenCodeSkillsDir) 'tuq-product-spec\SKILL.md' }
    }
    if (-not $file -or -not (Test-Path -LiteralPath $file)) { return $null }
    $m = Select-String -LiteralPath $file -Pattern '^  tuq-release:\s*(\S+)' | Select-Object -First 1
    if ($m) { return $m.Matches[0].Groups[1].Value }
    return $null
}

# --- main -----------------------------------------------------------------------------------------------------------

function Install-Tuq([string[]]$Arguments) {
    $script:Tool = ''
    $script:From = ''
    $script:Source = ''
    $i = 0
    while ($i -lt $Arguments.Count) {
        $a = $Arguments[$i]
        $value = if ($i + 1 -lt $Arguments.Count) { $Arguments[$i + 1] } else { $null }
        switch -Regex ($a) {
            '^--?tool$' { if ($null -eq $value) { Fail '--tool needs a value' }; $script:Tool = $value; $i += 2; continue }
            '^--?from$' { if ($null -eq $value) { Fail '--from needs a folder' }; $script:From = $value; $i += 2; continue }
            '^--?source$' { if ($null -eq $value) { Fail '--source needs a value' }; $script:Source = $value; $i += 2; continue }
            '^(-h|--help|-\?)$' { Usage; return }
            default { Fail "unknown argument: $a`n$(Usage)" }
        }
    }
    $script:Tool = $script:Tool.ToLowerInvariant()
    $toolName = @{ claude = 'Claude Code'; codex = 'Codex'; copilot = 'GitHub Copilot'; opencode = 'OpenCode' }[$script:Tool]
    if (-not $script:Tool) { Fail "say which tool: --tool claude|codex|copilot|opencode`n$(Usage)" }
    if (-not $toolName) { Fail "unknown tool '$($script:Tool)': use claude, codex, copilot or opencode" }
    # The tool is checked first, so nothing is installed when it is missing.
    switch ($script:Tool) {
        'claude' { $null = Need-Tool 'claude' 'https://claude.com/claude-code' }
        'codex' { $null = Need-Tool 'codex' 'https://developers.openai.com/codex/cli' }
        'copilot' { $null = Need-Tool 'copilot' 'https://github.com/features/copilot/cli' }
    }
    if ($script:From) {
        if (-not (Test-Path -LiteralPath $script:From -PathType Container)) { Fail "--from $($script:From) isn't a folder" }
        $script:From = (Resolve-Path -LiteralPath $script:From).ProviderPath
    }
    if (-not $script:Source) { $script:Source = $Repo }
    if (Test-Path -LiteralPath $script:Source -PathType Container) {
        $script:Source = (Resolve-Path -LiteralPath $script:Source).ProviderPath
    } elseif ($script:Source -match '^(\.|[A-Za-z]:\\|\\\\)') {
        Fail "--source $($script:Source) isn't a folder"
    }
    # The commands come from the releases of the repository the skills come from, when that is a GitHub one.
    $script:ReleaseRepo = if (Test-GitHubRepo $script:Source) { $script:Source } else { $Repo }
    if (-not $env:LOCALAPPDATA) { Fail 'LOCALAPPDATA isn''t set' }
    $script:BinDir = Join-Path $env:LOCALAPPDATA 'tuq\bin'
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $script:Tmp = Join-Path ([IO.Path]::GetTempPath()) ("tuq-install-" + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $script:Tmp | Out-Null
    try {
        Say "Installing tuq for $toolName"
        Install-Commands
        Add-ToUserPath
        Say "Installing tuq's skills in $toolName"
        switch ($script:Tool) {
            'claude' { Install-Claude }
            'codex' { Install-Codex }
            'copilot' { Install-Copilot }
            'opencode' { Install-OpenCode }
        }
        $version = Get-SkillsVersion
        Say ''
        if ($version) {
            Say "tuq $version is installed for $toolName (commands $($script:CommandsVersion) in $($script:BinDir))."
        } else {
            Say "tuq is installed for $toolName (commands $($script:CommandsVersion) in $($script:BinDir))."
        }
        if ($script:Tool -in 'claude', 'codex') {
            Say 'Its skills are tuq:<name>: /tuq:product-spec in Claude Code, $tuq:product-spec in Codex.'
        } else {
            Say 'Its skills are tuq-<name> there: /tuq-product-spec.'
        }
        Say 'Open a new terminal to use tuq and tuq-plan. Run this again to update tuq.'
    } finally {
        Remove-Item -LiteralPath $script:Tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

try {
    Install-Tuq $args
} catch {
    $reason = if ($_.Exception.Data['tuq']) { $_.Exception.Message } else { "$($_.Exception.Message) ($($_.InvocationInfo.PositionMessage))" }
    if ($RunAsFile) {
        [Console]::Error.WriteLine("tuq install: $reason")
        exit 1
    }
    # Run from a script block (irm | the command line): fail without closing the user's window.
    throw "tuq install: $reason"
}
