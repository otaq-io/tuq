---
name: product-spec
description: >-
  Settle what a new feature or a change to what the product does should be, before any code: ask the user one
  product question at a time (never about the implementation) until every function and edge case is decided, and
  save each agreed point as a numbered decision in tuq/decisions/ after checking it against the earlier ones.
  Use when the user starts describing or exploring a feature ("I want users to...", "what if we added...", "let's
  spec this"). Not for bug fixes, questions about existing code, refactors, reviews or other technical work.
argument-hint: "[feature or topic]"
---

# Settle what to build: tuq:product-spec

**Run `tuq rules` and read all of it before you read or write `tuq/`.** It holds the folder's layout,
numbering, the decision file's shape, statuses, the conflict check and renumbering; this skill doesn't repeat them.
`tuq` is a command tuq's installer puts on the PATH (`rules`, `template`, `init`, `next`, `index`, `clashes`), so
this skill names no file paths. `tuq: command not found` means tuq's commands aren't installed or the terminal
predates them: stop and say so.

No code has been written for the feature yet. Your job is to settle **what to build, for whom and why** with the
user, then write it down as the project's own **product decisions and docs**. After you, `tuq:tech-spec` works out
how to build it from what you wrote, and `tuq:milestone-plan` turns both into a plan.

The decisions carry the reasons to the next steps: write each one so someone who never saw this conversation
understands what was decided and why.

## Steps

1. **Get the context first:** the user's request (the skill's argument, or what they just said), `CLAUDE.md`,
   `AGENTS.md` and the project's notes, `tuq/README.md` and the decisions about this feature, and the code the
   feature touches (read it, don't change it). The repository root is `git rev-parse --show-toplevel`; outside a git
   repository use the current directory and say so.
2. **Find where the product decisions and docs live.** Product decisions always live in `tuq/decisions/` at the
   repository root, indexed in `tuq/README.md`, by `tuq rules`; read `tuq/README.md` and the
   decisions about this feature. The project's notes may name other product docs (an overview such as DESIGN.md):
   keep those in step too. Don't ask the user where decisions go.
3. **Find the open points.** Read the request, the existing decisions and docs, and the code it touches, then list
   for yourself what is undecided about the product: what is asked, **for whom** (which users or roles) and **why**
   (the problem it solves), which screens and data, what happens at the edges (empty, too many, errors, signed out),
   what is out of scope, and how the user will know it works. A decision the new work contradicts is an open point too.
   Put the ones that change what gets built first. Drop anything the request, the decisions, the code or the
   project's notes already answer. How to build it is Tech spec's job: leave it out. **Never ask about the
   implementation** (libraries, APIs, storage, file layout): only what the product does.
4. **Ask one question at a time** (see "Asking"), most important first, then wait for the answer. Read it. **When it
   settles a point, save that point now** (step 5: conflict check, then the decision file), then check your list
   again (an answer can settle or drop the next point), and ask the next question. Say where you are: "Question 2 of
   about 4". Once nothing that changes what gets built is open, or the user says "enough" or "go", go straight on to
   step 5. **No open points at all?** Then ask nothing and go straight on to step 5: there is nothing to wait for.
   **The user says it is getting more complicated than it should be, or isn't sure about the set of decisions?**
   Offer a step back ("Want me to take a step back with `tuq:step-back` and look at the feature without its
   decisions?") and call the Skill tool with `tuq:step-back` only after a yes.
5. **Write the product decisions and docs**, one point at a time, as soon as each is agreed, by `tuq rules`:
   - **Conflict check first** (`tuq rules`, "The conflict check"). On a conflict, don't save: the conflict question
     is your next question (step 4), and you wait for the answer.
   - Then `tuq next`, write `tuq/decisions/NNN-<slug>.md` from `tuq template decision` (no `Ticket:` line when
     there is no ticket), mark a superseded decision's Status line only, and run `tuq index`. In a repo with no
     `tuq/` yet, run `tuq init` first and add the pointers `tuq rules` describes (to each of `README.md`,
     `CLAUDE.md` and `AGENTS.md` that exists).
   - A point you saved earlier in this session and haven't committed is edited in place when the user changes it;
     anything committed changes only through a new, superseding decision.
   - Update the overview docs the project's notes name to match. You may edit any file the spec needs (an overview,
     a docs page, a sample), but don't build the feature.
   - **Never commit, stage or push**: the user commits when they choose.
6. **End** by listing every file you created or changed (decision numbers, the README, superseded files) and the
   points agreed, in a few lines.

## Asking

Ask with the AskUserQuestion tool, one question per call, then wait for the answer. The context and "Question 2 of
about 4" go in the question text. When the question has a few clear alternatives, offer them as **2–4 options** so
the user can answer with one key, give each option a **description with its pros and cons**, and recommend one when
there is a sensible default (first, with " (Recommended)" at the end of its label). For "Question 1 of about 4. Who
can open a shared wishlist link?":

- `Anyone with the link (Recommended)`: Easy to send in a chat; anyone who gets the link sees the list.
- `Signed-in friends`: Private, but each friend needs an account first: fewer people will look.

An **open question** ("Question 2 of about 4. Who asked for wishlist sharing, and what do they want to do with it?")
stays free text, with no options: it is plain text in your reply; then end your turn.

- **One question at a time.**
- 2–4 options, each a short label (at most 40 characters). The context goes in the question, each option's pros and
  cons in its description: one or two short sentences (at most about 200 characters) on what it costs and what it
  changes. Give every option one.
- Never invent options for an open question.
- The project's notes take precedence over this guidance.

## Rules

- Settle the product, not the code: don't plan the code or start building, and never ask about the implementation.
- Product decisions go in `tuq/` by `tuq rules`; read them before you touch the folder.
- Ask before you decide anything that changes what gets built; don't guess an answer the user hasn't given.
- **Never commit, stage or push, never open a PR, never merge**: the user commits when they choose.
