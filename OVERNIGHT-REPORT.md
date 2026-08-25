# Overnight report — issue #302

Three branches, each off `main`, each independently green against all four CI
gates (`fmt --check`, `clippy --all-targets --all-features -D warnings`,
`test --all-targets --all-features`, `test --doc`).

| Branch | Commits | Tests |
| --- | --- | --- |
| `model-profile-schema` | 3 | 1,851 (main: 1,847) |
| `model-profile-claude` | 1 | 1,863 |
| `model-profile-surface` | 3 | 1,871 |

Nothing was pushed to `bakedbean/workspacex`. No PR, issue or comment was
opened there.

## What landed

### Slice 1 — `model-profile-schema`

The reported bug, fixed. `model` and `provider` columns on `workspace_agents`
(migration 23); `ModelSelection` resolved from that row and threaded through
`SessionManager::spawn` → `spawn_session` into every per-agent builder; the
creating process's `WSX_*_MODEL` / `WSX_*_PROVIDER` captured onto the primary
agent row by `wsx workspace create`.

Verified end to end, not just by unit test: creating with `WSX_OMP_MODEL` set
now leaves `omp|qwen3.8-27b|local` on the row after the creating process has
exited, and creating without it leaves NULL so the ambient fallback still
governs.

### Slice 2 — `model-profile-claude`

`model_profiles`, a named-endpoint registry parsed like `shared_hosts`, and a
`model_profile` column (migration 24) naming which entry an instance uses.
Claude Code gained the model override it never had (`WSX_CLAUDE_MODEL`) plus
`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN` and
`CLAUDE_CODE_MAX_CONTEXT_TOKENS` from a profile — the four things a local
llama.cpp/ollama server actually needs.

### Slice 3 — `model-profile-surface`

`workspace create --profile <name>`, and `wsx agent profile <name|--clear>` to
change or drop the pin afterwards. Both refuse an unknown name and list the ones
that exist.

Display on three surfaces: `wsx agent list` and the agents panel show each
instance's selection in brackets; a new `model` detail-bar module shows the
primary's. In the panel, `p` cycles the primary through the configured profiles
and back off the end.

## Decisions the prompt did not make

**Capture on the CLI create path only, not the TUI's.** In the TUI the creating
and spawning processes are the same, so nothing is lost by leaving the value
ambient — and pinning it would stop an exported variable from applying to
workspaces after a relaunch, which is what someone exporting it process-wide
expects. Alternative rejected: capture everywhere, for uniformity.

**Fail fast on an unknown profile name at create/pin time, tolerate it at
spawn.** These look contradictory and are not: at create the user is present to
be told about a typo, and failing before the worktree exists saves them a
cleanup. At spawn nobody is present, and a profile renamed months later must not
make an existing workspace unopenable.

**Store the profile *name* on the row, not its expanded fields.** Editing a
profile then updates every instance pinned to it. Copying the values in would
freeze each workspace at the definition that existed when it was created.

**A profile without `model` defers to the recorded model** rather than clearing
it — "same model, different machine" is a normal thing to want from `base_url`
alone.

**Credentials rejected by key name, not by sniffing values.** A heuristic over
values would both miss ordinary-looking tokens and reject models whose names
look secret. `auth_token`, `token`, `api_key`, `apikey` and `password` are
refused by `config set` with a message naming `auth_token_env` as the
alternative.

**An unset `auth_token_env` variable is not an error.** A server on localhost
usually wants no token, and failing there would make the common local case the
awkward one. It logs at debug and spawns without one.

**`--model` re-asserted for claude on resume**, unlike omp's `-c` path. The pin
belongs to the workspace, so a resumed session should continue on the model the
workspace is pinned to. Endpoint variables are re-applied on every spawn
regardless, since they live in the process and not in the transcript.

**Slice 1 is three commits, slices 2 and 3 are one each.** Slice 1 had real
seams (storage / resolution / capture) where each part compiles and is
meaningful alone; forcing seams into the others would have meant committing code
only to replace it in the next commit.

## Deviations from the brief

**The profile is not on the dashboard row**, because that column is not text.
`agent_strip_spans` renders one coloured `▎` bar per live agent into a fixed
handful of cells; there is no room for a name "beside the agent" without
redesigning the column and the alignment every other column depends on.

It went in the detail bar instead, which is the surface this project already has
for per-workspace facts. The label is resolved from `app.agent_roster` — a cache
the app already rebuilds each reload — so the draw path performs no query, which
is what a context documented as "zero allocations per draw" requires.

**The panel sets the profile by cycling, not by picking.** `p` walks the primary
through the configured profiles and back off the end. A picker would mean a
second modal stacked on a modal for a list that is usually two entries long, and
this codebase already resolves bounded choices by cycling (`o`, `G`).

## Not done

**Slice 4 (endpoint health and contention warnings).** Gated on 1–3, which are
now complete, but not started.

## What I would do next

1. Cache the primary instance per workspace in `App` so the detail bar can show
   the model without a per-frame query, then add the detail module.
2. Extend profile resolution beyond claude to pi/hermes/codex/omp. The
   `ModelSelection` fields and the resolver are already agent-agnostic; only the
   four builders need to consume `base_url`/`auth_token_env`, and each expresses
   an endpoint differently.
3. Then slice 4. The contention warning is the genuinely novel part and nothing
   else in the stack can know it: three workspaces on one local endpoint queue on
   one GPU and split its context budget, which is the opposite of what a
   dashboard built to encourage parallelism implies.

## Caveat worth flagging

The original reproduction in issue #302 used `WSX_CLAUDE_BIN`, which is a binary
path rather than a model, and it is **still ambient** — that variable was the
stand-in used to demonstrate the mechanism, not the thing being fixed. The
mechanism it demonstrated (create-time environment never reaching the agent) is
fixed for model and provider. If binary overrides should be per-workspace too,
that is a separate change.
