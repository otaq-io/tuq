---
name: tuq-milestone-plan
description: >-
  Split a feature into the smallest pieces that each deliver one behaviour, with what waits for what, and write an
  orchestrator prompt for the agent that builds them: saves tuq/plans/<feature>.md and
  tuq/prompts/<feature>-orchestrator.md from the feature's decisions and tech spec. Runs when the user types
  /tuq-milestone-plan.
argument-hint: "[feature]"
disable-model-invocation: true
metadata:
  tuq-release: 2026.11.14
---

# Turn the specs into a plan: tuq-milestone-plan

**Run `tuq rules` and read all of it before you read or write `tuq/`.** It holds the folder's layout and rules;
this skill doesn't repeat them. `tuq` and `tuq-plan` are commands tuq's installer puts on the PATH
(`tuq-plan new`, `check`, `order`; each takes the `<feature>` name, see "Writing the plan"), so this skill names no
file paths. `command not found` means tuq's commands aren't installed or the terminal predates them: stop and say so.

`tuq-product-spec` settled what to build and `tuq-tech-spec` how. Your job is to turn them into work: the feature's
**plan**, which says why the work is split the way it is and lists its **pieces**, their order and what each waits
for, and the **orchestrator prompt** for the agent in charge, which starts a helper for each piece and merges their
work.

## Steps

1. **Get the context first:** the user's request (the skill's argument names the feature), `CLAUDE.md`, `AGENTS.md`
   and the project's notes, `tuq/README.md`, the decisions about this feature, its tech spec in
   `tuq/tech-specs/<feature>.md`, and the code the work touches (read it, don't change it). The repository root is
   `git rev-parse --show-toplevel`; outside a git repository use the current directory and say so. The project's
   notes may hold rules for the plan (for example "pieces that touch one crate each"): they apply to every step below.
2. **Decide the pieces**: **a piece is one behaviour**, one thing a user (or a caller) can see working
   when it is done ("a student can remove a book from a reading list"). It may touch every layer that behaviour needs
   (data, API, screen) and brings its own tests.
   - **List the behaviours first.** Read the product decisions about this feature (`tuq/decisions/`, indexed in
     `tuq/README.md`) and the tech spec, and write down for yourself every behaviour they ask for, one line each.
     Then map each behaviour to exactly one piece, so none is dropped or built twice.
   - **Split whenever there is more than one behaviour**, even when they share files. **Never split one behaviour by
     layer** (an API piece and a screen piece for the same behaviour). Shared groundwork that several behaviours need
     first (for example a new data model) may be its own piece, which they wait for, when it belongs to none of them.
   - **A piece that crosses layers says so** in its details ("one behaviour across the API and the page, on
     purpose"): a helper that measures a piece by one layer might otherwise hand it back as too big.
   - **Every piece's details end with a `Check:` line**: how to see its one behaviour working on its own (a test, a
     page, a request).
   - **One piece when the work is one**: a small feature gets a plan with a single piece.
   - **Order them by what each needs**: a piece waits for the pieces whose code it builds on ("the button waits for the
     API"). Pieces that wait for nothing, or for the same pieces, are built side by side, so don't chain what doesn't
     need chaining.
   - **Too big for one plan?** When the work is really several deliverables, each worth building and merging on its
     own, ask once whether to plan them as several features (see "Asking"); on a yes, each gets its own pair of
     files, under its own feature name.
3. **Write the plan** (see "Writing the plan"): first the why (how the work is split and the order, in a few lines,
   naming the decisions it follows), then the **Pieces** list in its fixed shape.
4. **Write the orchestrator prompt** next to the plan, in `tuq/prompts/<feature>-orchestrator.md`. `tuq-plan new`
   created it from the template in step 3; fill in only the feature's parts, and never repeat a piece's details there
   (the plan holds them):
   - the intro under the header: what the feature is, in a line, and the header's Ticket, Date, Tech spec and
     Decisions lines (the same as the plan's);
   - **Notes per piece**, as `### Piece N` headings (by number, never by title), only for pieces that need one: what
     the orchestrator passes on or watches for beyond the plan's details. Delete the template's `### Piece N` stub
     when no piece needs a note;
   - **Bringing it together**: what proves the whole feature works after the last merge, beyond each piece's
     `Check:` line.
   - Leave "Run order" and "Running a piece" alone: `tuq-plan order` writes the run order, and "Running a piece" is
     the same for every feature.

   Then run `tuq-plan order "$F" && tuq-plan check "$F"` again; `check` must pass. **Never commit, stage or push**:
   the user commits when they choose.
5. **End** by naming the two files (`tuq/plans/<feature>.md`, `tuq/prompts/<feature>-orchestrator.md`) and the
   number of pieces, and say that the orchestrator prompt is what to run next: hand it to the session that will build
   the feature.

## Writing the plan

The plan file `tuq/plans/<feature>.md` **owns the pieces**: the text between its plan markers is the plan, and the
run order in the prompt is generated from it. Never keep the pieces in step by hand.

**The feature name** (`$F` below) is the tech spec's: `tuq/tech-specs/<feature>.md` when it exists. Without one,
use the skill's argument in kebab case, or 2–5 kebab-case words from the topic (`reading-list-sharing`). When a plan
file of this feature already exists, use its name: planning again updates the same two files. **No tech spec in
`tuq/`**: read the file the project's notes or the user name for how it is built instead, and name that file in the
header's `Tech spec` line.

```sh
F=wishlist
tuq-plan new "$F"      # creates tuq/plans/$F.md and tuq/prompts/$F-orchestrator.md when missing
```

In the plan file, fill the header (Ticket, `none` when there is no ticket; Date; the Decisions and Tech spec it
builds on) and write the plan between `<!-- tuq:plan:start -->` and `<!-- tuq:plan:end -->`, nothing else there:

```markdown
<!-- tuq:plan:start -->

The wishlist is built in four pieces, one behaviour each: the share token first (groundwork the others
need), then saving and the page side by side, then the share link, which needs both. The agent in charge follows
tuq/prompts/wishlist-orchestrator.md.

## Pieces

1. Share token on wishlists — waits for nothing
   A random token per wishlist, created on first share: lib/wishlist.ts (decision 0141).
   Check: the token unit tests pass; an unknown token is rejected.
2. A customer saves an item — waits for 1
   The heart on each product card (components/ProductCard.tsx) and POST/DELETE /api/wishlist: one behaviour across
   the API and the card, on purpose.
   Check: tapping the heart saves the item; it is still saved after a reload.
3. A customer sees their wishlist — waits for 1
   /wishlist lists the saved items, with an empty state; GET /api/wishlist.
   Check: the page shows what was saved, and the empty state with nothing saved.
4. A customer shares their wishlist — waits for 2, 3
   The Share button on /wishlist and the read-only shared view, without the owner's email (decisions/0140).
   Check: the copied link opens the list signed out, with no email on the page.

<!-- tuq:plan:end -->
```

Then generate the run order and check the pair:

```sh
tuq-plan order "$F" && tuq-plan check "$F"
```

The shape is fixed, because the agent in charge reads it to start the helpers (`check` fails, naming the file and
line, when it isn't kept):

- The list is headed `## Pieces` and comes **last** in the plan: a heading after it ends the list.
- Each piece is numbered from 1, then its short title, then an em dash and what it waits for: "waits for nothing",
  "waits for 1", "waits for 2, 3". A piece only waits for pieces with a smaller number.
- Its details go on the lines under it, indented. **They must stand on their own**: a helper starts with a fresh
  context and reads only its piece, so name the files, screens and decisions it builds on, and anything its helper
  must not miss, so it doesn't have to explore again. Never statuses ("done", "in progress").
- The details end with the piece's `Check:` line, and the why names the orchestrator prompt's path, so whoever is
  handed only the plan finds it.

Every version of the plan file stays in git.

## Asking

Ask the user only what the specs don't answer and changes the plan (one feature or several, a piece the user may not
want). Ask with the AskUserQuestion tool, one question per call, then wait for the answer; the context goes in the
question text. When there are clear alternatives, offer **2–4 options**, each with a description of its pros and
cons, and recommend one when there is a sensible default (first, with " (Recommended)" at the end of its label). For
"The wishlist and the share link are each several pieces, and each could be built and merged on its own. Plan them
as one feature or two?":

- `One feature, 6 pieces (Recommended)`: One review and one merge; the share link waits for the whole wishlist.
- `Two features`: Wishlist and Share link built and merged apart, each with its own plan.

An open question stays free text, with no options: it is plain text in your reply; then end your turn. The project's
notes take precedence over this guidance.

## Rules

- Plan the work; don't build it. The agent in charge and its helpers write the code from the plan.
- The plan file owns the pieces: the run order comes from `tuq-plan order`, and `tuq-plan check` passes before you
  end.
- **Never commit, stage or push, never open a PR, never merge**: the user commits when they choose.
