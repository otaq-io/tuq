# otaq-claude-skills

A Claude Code plugin marketplace with one plugin.

| Plugin             | Skills                                                             | Enable it                                   |
| ------------------ | ------------------------------------------------------------------ | ------------------------------------------- |
| `tuq`           | `tuq:product-spec`, `tuq:step-back`, `tuq:tech-spec`, `tuq:milestone-plan` | any repo, any stack (user or project scope) |

The skills call two commands, `tuq` and `tuq-plan`, so no skill names a file path. They are Rust programs built from
`commands/`, with `plugins/tuq/rules.md` and `plugins/tuq/templates/` built in, and need nothing else installed.

## Install

```sh
claude plugin marketplace add https://github.com/otaq-io/otaq-claude-skills.git       # or a local checkout: claude plugin marketplace add ./otaq-claude-skills
claude plugin install tuq@otaq-claude-skills --scope user
```

A repo can offer the marketplace to its team from its committed `.claude/settings.json`:

```json
{
  "extraKnownMarketplaces": {
    "otaq-claude-skills": {
      "source": { "source": "github", "repo": "otaq-io/otaq-claude-skills" },
      "autoUpdate": true
    }
  },
  "enabledPlugins": {
    "tuq@otaq-claude-skills": true
  }
}
```

After trusting the folder, each teammate still installs once: `claude plugin install <plugin>@otaq-claude-skills
--scope project`.

## Developing

Add the marketplace from your local checkout (`claude plugin marketplace add <path>`): plugins load in place, so an
edit takes effect at the next session or after `/reload-plugins`. The plugins carry no `version`, so every commit is a
new version for git installs.

The commands: `cargo build --release` and `cargo test` in `commands/` (the tests run each command on a scratch
repository; see `commands/tests/golden.rs`).
