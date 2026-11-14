---
name: tuq-tech-spec
description: >-
  Work out how to build a feature whose product decisions are settled in tuq/decisions/: read them and the code,
  draft the architecture (where it goes, the parts it adds or changes and how they depend on each other, data and
  APIs, alternatives considered) with a very small plain-text diagram, discuss any part the user questions until
  it is settled, and save tuq/tech-specs/<feature>.md. Runs when the user types /tuq-tech-spec. Technical questions
  only, never the product.
argument-hint: "[feature]"
disable-model-invocation: true
---

# Work out how to build it: tuq-tech-spec

**Run `tuq rules` and read all of it before you read or write `tuq/`.** It holds the folder's layout and the
shape of the decisions you build on; this skill doesn't repeat them. Start the tech spec from `tuq template
tech-spec`. `tuq` is a command tuq's installer puts on the PATH, so this skill names no file paths.
`tuq: command not found` means tuq's commands aren't installed or the terminal predates them: stop and say so.

`tuq-product-spec` has settled **what** to build with the user, as numbered decisions in `tuq/decisions/`. Your job
is to work out **how** to build it, and write that down as the feature's **tech spec**, `tuq/tech-specs/<feature>.md`.
After you, `tuq-milestone-plan` turns both specs into a plan.

The tech spec carries the reasons to the next steps: write each decision so someone who never saw this conversation
understands what was decided, what else was considered and why.

## Steps

1. **Get the context first:** the feature (the skill's argument, or what the user just said), `CLAUDE.md`,
   `AGENTS.md` and the project's notes, `tuq/README.md` and the decisions about this feature, and the code the
   feature touches (read it, don't change it). The repository root is `git rev-parse --show-toplevel`; outside a git
   repository use the current directory and say so.
2. **Read the decisions.** Read every decision about the feature in full: they are what you build on. Read
   `tuq/README.md`'s index too, and the earlier decisions that touch the same feature. **No decisions about the
   feature:** ask once whether to run `/tuq-product-spec` first (recommended) or go on from the description, with
   `Decisions: none`.
3. **Find the tech spec file.** It is always `tuq/tech-specs/<feature>.md` at the repository root; don't ask the
   user where it goes.
   - **The feature name** is the skill's argument in kebab case, or 2–5 words from the topic
     (`reading-list-sharing`); reuse an existing `tuq/tech-specs/<feature>.md` or `tuq/plans/<feature>.md` of the
     same feature, so the feature's tech spec, plan and prompt match by name, and update that file.
   - **No `tuq/` yet:** run `tuq init` and add the pointers `tuq rules` describes (to each of `README.md`,
     `CLAUDE.md` and `AGENTS.md` that exists).
4. **Work out how to build it: draft first, then discuss.** Read the code the work touches and decide: where the new
   code goes, the parts it adds or changes and how they depend on each other, the data it stores and how, the APIs
   and their shapes, what it reuses, how it is tested, and what could go wrong (migrations, load, security). Shape
   the design as "Shaping the design" says: start from the caller, design it twice when the shape matters, and check
   it against the warning signs.
   - **Draft:** write the **whole** tech spec file (see "The tech spec file") before you ask anything: every choice
     made, with its reason. Don't walk the user through the choices one by one first.
   - **Show and ask** (see "Asking"): the rough explanation in a few lines, the diagram, the architecture decisions
     as a numbered list of one line each, and the file's path go in your reply, then ask "What do you want to
     question or change?".
   - **Discuss:** when the answer questions or changes a point, revise the file in place, and ask the same closing
     question again, starting with your answer to that point (why the draft chose it, or what changed now). **One
     point per turn.** A point that changes the shape (a new module, a different store) runs "design it twice"
     again for that point and keeps the loser under "Considered".
   - **Settled** when the user says "settled", "enough", "go" or the like: go straight on to step 5. The file is kept
     current after each answer, so an interrupted session loses nothing.
   - **Technical only.** Never ask about the product: it is settled in `tuq/decisions/`. A point that turns out to
     be an open product question is not decided here: say so in your next question, write it under "Open product
     points" in the file, suggest `/tuq-product-spec`, and design around it only where the design doesn't depend on
     the answer.
5. **Write the tech spec** in `tuq/tech-specs/<feature>.md`. You may edit any file the spec needs (a schema sketch,
   a config sample), but don't build the feature. **Never commit, stage or push**: the user commits when they choose.
6. **End** by naming the file you wrote and the main choices in a few lines, and any open product points, and suggest
   `/tuq-milestone-plan <feature>` next.

## The tech spec file

Start from `tuq template tech-spec` and fill it in. Rough on purpose: the explanation fits on
one screen and the whole file a few screens. It is the spec, not the plan: no step-by-step coding plan, no code.

- **Header exactly three lines:** `Ticket` (the ticket key; left out when there is none), `Date` (the day of the
  latest change), `Decisions` (the numbers it builds on, comma-separated, or `none`). No other header line:
  Milestone plan reads this shape.
- **How it is used:** the caller sketch of "Shaping the design".
- **How it is built:** a few short paragraphs: where the change goes, the parts it adds or changes, the data and
  APIs, what it reuses.
- **What changes: the diagram owns the dependencies.** One very small plain-text drawing in a ` ```text ` block:
  the parts the change touches and who calls or imports whom, each marked `[new]` or `[changed]` (untouched parts
  unmarked, and only where the change plugs in), a handful of lines. Find the dependencies by reading the imports and
  call sites of the touched files (`grep` for the language's import, require or use lines, or the LSP tool). No
  Mermaid, no images. The **Parts** table under it gives each part's path, new or changed, and role, but **no
  dependency column**: the dependencies are written once, in the diagram.
- **Architecture decisions:** numbered sections (`### 1. …`), each: what was chosen, why, and "Considered: … Lost:
  …" when there was a real alternative (always for a "design it twice" point). These are the numbered points of the
  discussion.
- **Testing** and **Risks:** two short sections.
- **Open product points:** only when there are any (step 4).

## Shaping the design

1. **Start from the caller.** Before you decide the shape, write a few lines of how the new code will be used: the
   call site, the command the user types, or the request and its response. Derive the types and the module from it,
   not the other way round. Put the sketch in the doc, so Milestone plan, the builders and the user see the
   shape at a glance:

   ```rust
   let link = wishlists.share(list_id)?;   // ShareLink { token, url }
   let list = wishlists.open_shared(&token)?;  // read-only Wishlist, or NotShared
   ```

2. **Design it twice when the shape matters**: the feature adds a new module, a new stored data shape, or an API
   other code will call. Then sketch two whole shapes, not two variants of one ("a share token column in
   `wishlists`" against "a separate `shares` table"), pick one, and write the other in the doc's "considered" part
   with why it lost. A smaller change needs one design, as before.
3. **Check the chosen design against the warning signs.** The next contributor is an agent that sees only the files
   it opened, copies the nearest example and takes the shortest path that compiles; prefer the design where a change
   that looks right from one file is right for the whole repo. Fix each sign that fires; when you keep one on purpose,
   the decision says why. Don't list the ones that passed.
   - **Two writers of one state**: more than one module writes the same state or keeps its own copy. Give each
     piece of state one owner; the others read it or ask the owner.
   - **Two ways to do one task**: every way keeps gaining callers. Keep one.
   - **Lists kept in step by hand**: adding an item means editing several places. Keep one list and derive the
     rest, or make the build fail when they disagree.
   - **Pass-through layers**: a layer that only forwards the same arguments. Remove it, or give it real work.
   - **Shallow modules**: callers call several functions to do one thing, or the interface exposes its internal
     steps. Hide more behind a smaller interface.
   - **A decision leaking into several modules**: a storage format, wire type or policy that several modules
     depend on, so changing it means coordinated edits. Keep it behind one module.
   - **Internals other modules can import**: they become part of the interface. Make them private, so the import
     fails.

## Asking

The draft's closing question (step 4) carries the short version in your reply; the file holds the rest:

````text
Tech spec draft: tuq/tech-specs/reading-list-sharing.md

A random share token on each list. ListsService gets share() and openShared(); a new read-only SharedListPage opens
a list by its token. No new service.

```text
ShareButton [new] --> ListsService [changed] --> ListsApi [changed]
SharedListPage [new] --'                          lists table: + share_token [changed]
```

1. A token column on `lists`, not a `shares` table
2. openShared() returns NotShared instead of throwing
3. The shared page reuses the list view in read-only mode
````

Then the AskUserQuestion tool asks "What do you want to question or change?" with the options
`Settled (Recommended)` and `Change something`; the user types the point's number and what bothers them under
"Other". The next reply starts with your answer: "1. Kept the token column: a list has one link, so a table adds a
join for nothing. …", then asks the same question again.

Any other question also goes through AskUserQuestion, one per call. When it has a few clear alternatives, offer them
as **2–4 options** so the user can answer with one key, give each option a **description with its pros and cons**,
and recommend one when there is a sensible default (first, with " (Recommended)" at the end of its label). For
"Question 1 of 2. A share link is a random token in the wishlists table. Should it expire?":

- `Links never expire (Recommended)`: Simplest; an old link keeps working until the list is deleted.
- `Expire after 30 days`: Safer, but friends get a 'link expired' page and ask for a new one.

An **open question** ("Question 2 of 2. How many wishlists does your largest customer have today, roughly?") stays
free text, with no options: it is plain text in your reply; then end your turn.

- **One question at a time.**
- 2–4 options, each a short label (at most 40 characters). The context goes in the question, each option's pros and
  cons in its description: one or two short sentences (at most about 200 characters) on what it costs and what it
  changes. Give every option one.
- Never invent options for an open question.
- The project's notes take precedence over this guidance.

## Rules

- Decide how to build it; don't build it. The code is written later, from your decisions and the plan.
- Don't reopen what Product spec settled with the user. When the product decisions can't be built as written, ask the
  user, with the options you see.
- **Never commit, stage or push, never open a PR, never merge**: the user commits when they choose.
