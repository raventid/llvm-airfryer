# Architecture

## Core idea

**The CLI is the tool. The TUI is a UI on top of the CLI.**

At its core, `llvm-airfryer` is a parameterized command-line tool: every action
is fully described by a long list of explicit flags.

```
llvm-airfryer llvm build \
    --repo /path/to/llvm-project \
    --branch my-patch-branch \
    --build-dir build-airfryer-my-patch \
    --install-dir /opt/llvm-builds/my-patch \
    --jobs auto \
    --checkout \
    --pull
```

This invocation is self-contained, reproducible, and scriptable. An expert user
who already knows the state of their system can type it directly. CI agents and
LLMs do exactly this.

## What the interactive UI is for

The dialoguer-based menu is **an alternative front-end that crafts the same
invocation by asking questions**. Each prompt corresponds to one CLI flag:

| Prompt | Flag |
|---|---|
| "Path to llvm-project directory?" | `--repo <path>` |
| "Branch to build (type to search)" | `--branch <name>` |
| "Which LLVM build to use?" | `--llvm-build <id>` |
| "Open in browser?" | `--open` |

A finished interactive walk-through is conceptually equivalent to having typed
the full flagged command up front.

## Why the interactive UI still has value

It isn't just convenience — it does work the user would otherwise do by hand.
**Its primary job is gathering state the user doesn't have in their head.**

1. **Inspects the live system on the user's behalf.** Lists git branches,
   finds existing LLVM builds in `builds/`, detects whether Compiler Explorer
   is already running and on which port, parses Zig's expected LLVM version,
   checks whether the current build is stale relative to its source branch,
   etc. A human cannot reliably remember "CE is running on port 8003" or
   "the `vpternlog` build was last compiled three days ago" — the TUI looks
   it up so they don't have to.
2. **Turns gathered state into smart defaults.** "CE is already running on
   `127.0.0.1:8003` — reuse it?" is a default that only exists because the
   tool just probed the system.
3. **Validates inputs interactively.** Path doesn't exist → reprompt instead
   of failing the whole command.
4. **Recovers from in-progress state.** Mid cherry-pick? Offer continue / abort.

### Human users vs. agent users

The asymmetry is the whole point:

- A **human** doesn't remember which port CE is on, which branches exist,
  whether the build is stale, or which LLVM version Zig expects. The TUI
  collects this for them and offers it as defaults.
- An **LLM or CI agent** can record and pass that state through context. It
  doesn't need the TUI to gather it — it already has the facts and can write
  the full flagged command directly.

This is why the same tool needs both surfaces. The CLI assumes you already
know the state; the TUI assumes you don't and figures it out for you. A user
who has already done the analysis themselves can skip the TUI:

```
llvm-airfryer ce run --port 8003 --reuse-if-running
```

Implementation corollary: every piece of state the TUI gathers must also be
**queryable from the CLI** (e.g. `llvm-airfryer ce status --json`,
`llvm-airfryer llvm builds --json`). Otherwise agents are forced to either
parse TUI output or duplicate the probing logic.

## Long-running and continuous work belongs in a daemon

The CLI and TUI are short-lived: they take user intent, hand it off, and
exit. Anything that has to keep running — scheduled rebuilds, file-watch
rebuilds, periodic test runs, "rebuild whenever upstream changes" — belongs
in a separate **daemon process** that the CLI/TUI spawns and supervises.

```
┌──────────────────┐         ┌─────────────────────┐
│  CLI / TUI       │ control │  llvm-airfryer-d    │
│  (short-lived)   │────────▶│  (long-lived)       │
│  user intent     │  IPC    │  rebuilds, tests,   │
│                  │◀────────│  watches, schedules │
└──────────────────┘ status  └─────────────────────┘
```

Why split them:

- **Builds take 30+ minutes.** A user shouldn't have to keep their terminal
  open, and a TUI/CLI process that exits would orphan the work.
- **"Rebuild on every push to `main`"** isn't a command, it's a policy. A
  policy needs something running.
- **Tests should be able to run after a build completes**, automatically,
  without the user being present.
- **Status queries must be cheap.** `llvm-airfryer status` should answer in
  milliseconds — that means asking the daemon, not re-probing the system.

What the daemon owns:

- A queue of rebuild/test jobs.
- File watchers (e.g. `inotify`/`fsevents` on the llvm-project source).
- Schedulers (cron-like triggers).
- The authoritative answer to "is this build stale?", "what is CE's current
  port?", "what's the last test result?" — caching state the CLI/TUI used
  to probe ad-hoc.

What the daemon does **not** own:

- User dialogue. Prompts always live in the CLI/TUI process.
- Interactive recovery flows like cherry-pick conflict resolution.

Suggested CLI surface:

```
llvm-airfryer daemon start | stop | status | logs
llvm-airfryer build --branch foo --background       # enqueue, return job id
llvm-airfryer watch  --branch foo --rebuild --test  # register a policy
llvm-airfryer jobs   --json                         # list queue
llvm-airfryer status --json                         # daemon-cached snapshot
```

Implementation notes:

- The daemon is just another consumer of the same `Args` structs the CLI
  produces. "Run in the daemon" is an executor choice, not a different
  command set. `--background` on any action means "send this Args to the
  daemon instead of running it in-process."
- IPC can start as a Unix socket + JSON lines (or named pipes); the existing
  pidfile pattern at `src/main.rs:437` for Compiler Explorer is the seed of
  this — generalize it.
- Until the daemon exists, every command runs in-process. The architecture
  is forward-compatible: adding the daemon doesn't change the CLI surface,
  it just adds `--background` and the `daemon`/`watch`/`jobs` subcommands.

## TUI walkthroughs must echo their CLI equivalent

Once a user finishes an interactive walkthrough, the tool **prints the exact
CLI command that would reproduce the same result**, before (or alongside)
running the action.

```
$ llvm-airfryer
> Interesting Resources → Security

  ┌─ Equivalent command ─────────────────────────────────────┐
  │  llvm-airfryer resources --category security             │
  └──────────────────────────────────────────────────────────┘

  [resource list...]
```

This matters because:

- **No one wants to click through six menus to repeat a thing.** Once you've
  found what you want, you should be able to copy a one-liner and bind it,
  alias it, or paste it into a script.
- **The TUI becomes the discoverability layer for the CLI.** Users learn the
  flags by using them implicitly, then graduate to typing them directly.
- **It enforces the architecture.** If a TUI path can't be expressed as a CLI
  command, that's a bug in the design — there's a hidden prompt or implicit
  state somewhere that shouldn't be there.
- **It makes sharing trivial.** "How did you reproduce that build?" becomes a
  copy-paste of the echoed command, not a screen recording of menu clicks.

Implementation note: since both input layers converge on the same `Args`
struct (see Layering below), echoing the command is just `Args::to_cli()` —
serialize the resolved args back into flag form.

## Implications for the codebase

- **Every interactive prompt should map cleanly to a flag.** If a question
  can't be expressed as a flag, the design is wrong.
- **Action functions take a struct of resolved arguments**, not a `dialoguer`
  call site. The TUI populates that struct by prompting; the CLI populates it
  by parsing argv. Both call the same core function.
- **Defaults live in one place.** The TUI shows them as pre-filled answers;
  the CLI applies them when a flag is omitted.
- **System-state inspection (free ports, existing builds, current branch) is
  exposed as helper functions.** The TUI calls them to seed defaults; CLI
  users call them mentally and pass the result as a flag.
- **No prompts inside action functions.** Prompting belongs at the boundary
  (the menu layer), so CLI dispatch never deadlocks on stdin.

## Layering

```
┌─────────────────────────────────────────────────┐
│  argv (clap)        │  TUI menu (dialoguer)     │  ← input layer
├─────────────────────┴───────────────────────────┤
│  resolved Args struct (per subcommand)          │  ← contract
├─────────────────────────────────────────────────┤
│  action functions: build(), test(), run_ce()…   │  ← core logic
├─────────────────────────────────────────────────┤
│  system probes: git branches, free port, builds │  ← shared helpers
└─────────────────────────────────────────────────┘
```

Both input layers converge on the same `Args` struct. Action functions never
look up at the input layer — they only consume args and call helpers.
