# tuq

tuq is a set of agent skills for deciding what to build before building it, kept as plain files in a `tuq/` folder at
the root of your repository:

| Skill            | What it does                                                                                                   |
| ---------------- | -------------------------------------------------------------------------------------------------------------- |
| `product-spec`   | asks you one product question at a time until a feature is settled, and saves each answer as a numbered decision in `tuq/decisions/` |
| `step-back`      | looks at a feature's decisions from a distance and names the ones adding the most complication; writes nothing |
| `tech-spec`      | works out how to build a feature from its decisions and the code, and saves `tuq/tech-specs/<feature>.md`      |
| `milestone-plan` | splits a feature into the smallest pieces that each deliver one behaviour, and writes `tuq/plans/<feature>.md` and an orchestrator prompt |

It works in Claude Code, Codex, GitHub Copilot and OpenCode, on any stack. The skills call two small commands, `tuq`
(rules, templates, decision numbers, the index) and `tuq-plan` (plans), so no skill names a file path and nothing is
numbered by hand. They are Rust programs with the rules and templates built in, and need nothing else installed.

Skill names depend on the tool: `/tuq:product-spec` in Claude Code and `$tuq:product-spec` in Codex, where the plugin
name is the namespace; in Copilot and OpenCode each skill carries the prefix in its own name, so a repository's own
`product-spec` can never hide tuq's: `/tuq:tuq-product-spec` in Copilot (plugin, then skill; `/tuq-product-spec` is
an unknown command there), and `tuq-product-spec` in OpenCode, which you ask for by name.

## Install

One command per tool you use. It installs tuq the tool's own way, puts `tuq` and `tuq-plan` in `~/.tuq/bin`
(`%LOCALAPPDATA%\tuq\bin` on Windows) and adds that folder to your PATH. Run it again to update.

macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/otaq-io/tuq/main/install.sh | sh -s -- --tool claude    # Claude Code
curl -fsSL https://raw.githubusercontent.com/otaq-io/tuq/main/install.sh | sh -s -- --tool codex     # Codex
curl -fsSL https://raw.githubusercontent.com/otaq-io/tuq/main/install.sh | sh -s -- --tool copilot   # GitHub Copilot
curl -fsSL https://raw.githubusercontent.com/otaq-io/tuq/main/install.sh | sh -s -- --tool opencode  # OpenCode
```

Windows (PowerShell):

```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/otaq-io/tuq/main/install.ps1))) --tool claude
```

with `--tool codex`, `--tool copilot` or `--tool opencode` for the other tools. Then open a new terminal, and start a
new session in the tool.

What the installer runs for each tool:

| Tool           | First install                                                                               | Running it again                                                                          |
| -------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Claude Code    | `claude plugin marketplace add otaq-io/tuq`, `claude plugin install tuq@otaq-claude-skills --scope user` | `claude plugin marketplace update otaq-claude-skills`, `claude plugin update tuq@otaq-claude-skills` |
| Codex          | `codex plugin marketplace add otaq-io/tuq`, `codex plugin add tuq@otaq-claude-skills`        | `codex plugin marketplace upgrade otaq-claude-skills`, `codex plugin add tuq@otaq-claude-skills` |
| GitHub Copilot | `copilot plugin marketplace add otaq-io/tuq`, `copilot plugin install tuq@otaq-claude-skills` | `copilot plugin marketplace update otaq-claude-skills`, `copilot plugin update tuq@otaq-claude-skills` |
| OpenCode       | copies the `tuq-*` skill folders into `~/.config/opencode/skills` (`$XDG_CONFIG_HOME/opencode/skills` when set) | the same copy, over the old one                                                            |

The marketplace keeps its old name, `otaq-claude-skills`, so installs made before the repository was renamed keep
updating. Claude Code and Codex read `.claude-plugin/marketplace.json` (the `tuq` plugin in `plugins/tuq`); Copilot reads
`.github/plugin/marketplace.json`, whose `tuq` is the prefixed copy in `plugins/tuq-prefixed`. The commands come from
the newest GitHub release and are checked against its `SHA256SUMS`. The installer prints the version it installed; when
it can't install (the tool isn't on the PATH, a download fails or doesn't match), it says why and exits non-zero.

A repository can offer tuq to its team on Claude Code from its committed `.claude/settings.json`:

```json
{
  "extraKnownMarketplaces": {
    "otaq-claude-skills": {
      "source": { "source": "github", "repo": "otaq-io/tuq" },
      "autoUpdate": true
    }
  },
  "enabledPlugins": {
    "tuq@otaq-claude-skills": true
  }
}
```

Each teammate still runs the installer once, for the commands.

## How Otaq uses tuq

Otaq, a kanban board that drives coding agents, runs tuq's skills in its flow stages: a stage names
`tuq:product-spec`, and Otaq types it in the form the stage's tool uses. It uses the tuq you installed in each tool, the
same one your own terminal uses. When a stage needs a tuq skill that tool's tuq doesn't have, Otaq runs this installer
for that tool and checks again. It reads the installed version from the `metadata.tuq-release` line every tuq
`SKILL.md` carries, and shows it per tool and per session. Otaq adds `~/.tuq/bin` to its sessions' PATH itself and lets
`tuq` and `tuq-plan` run without asking.

## Developing

Add the marketplace from your local checkout (`claude plugin marketplace add <path>`): plugins load in place, so an
edit takes effect at the next session or after `/reload-plugins`. The plugins carry no `version`, so every commit is a
new version for git installs.

The commands: `cargo build --release` and `cargo test` in `commands/` (the tests run each command on a scratch
repository; see `commands/tests/golden.rs`).

`plugins/tuq-prefixed/` is the same plugin for GitHub Copilot and OpenCode, with each skill named `tuq-<name>`
(`/tuq:tuq-product-spec` in Copilot, `tuq-product-spec` in OpenCode). It is generated from `plugins/tuq/`: after editing a skill, run `scripts/release --generate` and
commit both; `scripts/release --check` (run by CI) fails when they differ.

The installers take two options for trying a release before it is published: `--from <folder>` installs the commands
from a folder holding the release assets (`dist/` after `scripts/release --dry-run`), and `--source <folder>` installs
the skills from a checkout instead of `otaq-io/tuq` (it also takes `owner/repo` or a git URL):

```sh
sh install.sh --tool codex --from dist --source .
powershell -ExecutionPolicy Bypass -File install.ps1 --tool codex --from dist --source .
```

## Releasing

`scripts/release` (on the maintainer's Mac) dates the release (`2026.11.14`, `.2` for a second that day), stamps
`metadata.tuq-release` into every `SKILL.md`, regenerates `plugins/tuq-prefixed/`, builds and signs `tuq` and
`tuq-plan` for macOS, Linux and Windows into `dist/`, commits, tags `v<version>`, pushes and creates the GitHub
release with the programs as assets. `scripts/release --dry-run` does all of it but the push and the release. The
script's header lists the steps, the asset names and what it needs installed.
