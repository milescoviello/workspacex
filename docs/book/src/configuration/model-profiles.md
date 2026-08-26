A **model profile** is a named endpoint plus a model, so a workspace can be
pointed at a particular server rather than at whatever the machine happens to
default to. It is what makes a local model — llama.cpp, ollama, vLLM — usable
from wsx, and it works for a hosted endpoint in exactly the same way, because
to wsx they differ only by a URL.

## Defining profiles

Profiles live in the `model_profiles` setting, one per line: a name, then
`key=value` fields.

```bash
wsx config edit model_profiles
```

```text
# a llama.cpp server on this machine
local-qwen  base_url=http://127.0.0.1:8091 model=qwen3.8-27b max_context=212992

# the same model on the box with the spare GPU
gpu-box     base_url=http://gpu-box.lan:8091 model=qwen3.8-27b auth_token_env=GPU_TOKEN
```

| Field | Meaning |
| --- | --- |
| `base_url` | API endpoint the agent should talk to. Must start with `http://` or `https://` |
| `model` | Model name to request |
| `provider` | Named local provider for agents pointed by name rather than URL: `ollama` or `lmstudio` |
| `auth_token_env` | **Name** of an environment variable holding the token |
| `max_context` | Context window to advertise, when it differs from the model's default. Must be greater than zero |
| `reasoning` | Reasoning effort to request: `none`, `low`, `medium`, `high`, `max`. Only codex reads it |

A profile must set at least one of `base_url` or `model`, or it would do
nothing. Blank lines and `#` comments are ignored.

### Write `base_url` as the bare server

`base_url` is the server itself — `http://127.0.0.1:11434`, the line ollama or
llama.cpp prints on startup — not an API path. Agents want different paths under
it, so wsx appends what each one needs: codex's local providers speak the
Responses API and post to `<base>/responses`, so wsx hands codex the
OpenAI-compatible root `<base>/v1`, while claude and pi take the bare URL.

A `base_url` that already has a path is passed through untouched, so a reverse
proxy prefix or an explicit `/v1` still works.

Everything is checked when the setting is written rather than when an agent
spawns. A `base_url` with no scheme, or a `max_context` of zero, can only be a
mistake — and left to spawn time it surfaces as an opaque failure inside an
agent, long after the person who typed it has moved on.

## Which agents can use which fields

These five do not point at an endpoint the same way, and the differences decide
what a profile can promise. Each row was established by reading the tool.

| Agent | `model` | endpoint | how |
| --- | --- | --- | --- |
| `claude` | yes | **`base_url`** | `ANTHROPIC_BASE_URL` in the environment |
| `pi` | yes | **`base_url`** | `LLAMA_BASE_URL` — llama.cpp servers |
| `codex` | yes | **`provider`** (+ optional `base_url`) | `--oss --local-provider ollama\|lmstudio`, redirected by `CODEX_OSS_BASE_URL` |
| `hermes` | yes | no | its `config.yaml` beats anything wsx can set |
| `omp` | yes | no | custom providers live in `~/.omp/agent/models.yml` |

**Codex takes a provider name, not a URL.** It has no flag for an arbitrary
endpoint; reaching one otherwise means writing a `model_providers` entry into
`~/.codex/config.toml`, which is not wsx's file to edit. Codex ships
`--oss --local-provider` for exactly this case, and a profile's `provider` field
selects it:

```text
local-ollama  provider=ollama model=qwen2.5:7b
```

That alone reaches the **default** local port. Adding a `base_url` redirects it
to another machine — which is the point when the local GPU is busy:

```text
gpu-box  provider=ollama base_url=http://gpu-box.lan:11434 model=qwen2.5:7b
```

wsx passes that as `CODEX_OSS_BASE_URL` — with `/v1` appended, since codex posts
to `<base>/responses` — which is the variable codex honours here; `OLLAMA_HOST`
and `OLLAMA_BASE_URL` are both ignored by this path. A `base_url` **without** a
`provider` still warns, because codex only consults it in `--oss` mode and would
otherwise look configured while changing nothing.

Codex also asks for `xhigh` reasoning by default, which ollama refuses for every
model it serves — `invalid reasoning value: "xhigh"` — so a local codex spawn
sends `model_reasoning_effort=none` unless the profile names something else:

```text
gpu-box  provider=ollama base_url=http://gpu-box.lan:11434 model=qwen3-coder reasoning=high
```

`none` is the fallback because it is the only value that also works for a model
with no thinking mode at all, which answers `"<model>" does not support thinking`
to anything else. A codex spawn that is *not* redirected to a local provider is
left alone — its effort is whatever the user configured.

All of this is re-applied when a session **resumes**, not just when it starts.
Codex does not restore a provider from a resumed session: without the flags, the
second attach of a locally-pinned workspace sends the local model's name to
OpenAI's own API on the user's account.

**Hermes and omp cannot be moved per spawn.** Hermes resolves its endpoint as
`argument or config.yaml or OPENROUTER_BASE_URL`, so the config file wins over
anything wsx can set and there is no flag; omp reads custom providers only from
its own `models.yml`. Both already reach any model through their own
configuration, which is why this is a limitation rather than a gap.

None of it is silent. The CLI warns when a profile carries an endpoint the
chosen agent cannot use, and wsx records no endpoint for that session — so the
dashboard never claims a workspace is on a server its agent never contacted.

```console
$ wsx workspace create backend --agent codex --profile local-qwen
pinned to model profile local-qwen
warning: profile 'local-qwen' sets base_url, but codex cannot be given an
arbitrary endpoint — set `provider=ollama` or `provider=lmstudio` instead
```

No token is forwarded to `pi`: it reads no API-key environment variable, and its
only mechanism is `--api-key`, which would put the secret in the process list.

## Choosing a profile

**In the TUI** there are two places. On the dashboard, `?` opens the workspace
actions card and `m` cycles the selected workspace's model — the card stays open
so a third profile is a third keypress, not a third round trip. And when
creating, `n` opens the new-workspace modal where `^p` cycles the model, the
same way `tab` cycles the agent and `^s` toggles tmux sharing. The modal always
shows the line, chosen or not:

```text
name: add-widgets
agent: claude  [tab] toggle
model: local-qwen  [^p] cycles
shared (tmux): off — ^s toggles
```

Creation is the one moment a choice applies immediately, because nothing has
spawned yet.

**From the CLI**, at creation or afterwards:

```bash
wsx workspace create backend --name add-widgets --profile local-qwen
wsx agent profile local-qwen                    # the primary agent
wsx agent profile --agent claude#2 local-qwen   # a specific one
wsx agent profile --clear                       # back to the default
```

A name that does not resolve is refused, and the error lists the names that do.
On `create` that check runs before the worktree exists, so a typo costs nothing.
Clearing must be spelled `--clear`, so a half-typed command cannot silently
unpin a workspace.

## Child workspaces inherit the model

A workspace created from *inside* another one inherits its model, the same way
it already inherits yolo mode and the agent kind:

```console
$ wsx workspace create backend --name follow-up
created workspace backend/follow-up at …
pinned to model profile local-qwen
inherited yolo, model=local-qwen from backend/add-widgets
```

This matters because wsx's own agent doctrine instructs an agent to hand
independent work to a new workspace. Without inheritance, an agent deliberately
pinned to a local endpoint would spawn children that quietly went somewhere
else — and cost money.

An explicit `--profile` overrides it, and a pin whose profile has since been
deleted is not propagated: spreading a dangling reference to every child is
worse than starting them on the default.

## Changing the model of a *running* agent

You cannot. A process's environment is fixed when it starts, so a pin applies at
the agent's **next spawn** — this is a property of processes, not a limitation
of the pin.

Both surfaces say so rather than looking inert. The agents panel (`Ctrl-x a`)
shows the live model and what will replace it:

```text
▎ claude  (primary)  [claude-opus → local-qwen next spawn]
▎ claude#2           [local-qwen]
```

and the [`model` detail-bar module](../daily-use/detail-bar.md) says the same
thing on its second line — it is off by default, so add it to a container first.
To
apply it now, restart that agent — archive and recreate the workspace, or kill
the agent so it respawns.

### Shared workspaces wait longer

A [tmux-shared](../integrations/shared-workspaces.md) workspace keeps its agent
alive inside a tmux server that outlives the client, and re-attaching runs
`tmux new-session -A`, which **attaches to the surviving session rather than
re-running the command**. A new model therefore does not arrive on re-attach the
way it does for a direct workspace — it waits for that tmux session to end.

Both surfaces say so: the CLI prints *"applies when this workspace's tmux
session is restarted, not on re-attach"*, and the detail bar reads
`other on tmux restart`. To apply it, end the session — `wsx workspace unshare`,
or kill it directly with `tmux kill-session -t wsx-<repo>-<slug>`.

`wsx agent list` shows what each instance is pinned to:

```console
$ wsx agent list
1  claude  (primary)  [local-qwen]
```

## Tokens are referenced, never stored

`auth_token_env` holds the *name* of an environment variable, and the token is
read from that variable at spawn time. Writing a literal token into a field is
refused:

```console
$ wsx config set model_profiles 'x base_url=http://h api_key=sk-live-…'
line 1: profile 'x' sets 'api_key' — store the NAME of an environment
variable in auth_token_env instead, never the token itself
```

`state.db` is an ordinary unencrypted file that gets backed up and copied along
with a home directory. A token written into it would be a credential at rest
that nothing knows how to rotate.

A variable that is unset is not an error — a local server usually wants no token
at all, so the agent simply spawns without one.

## How a selection is resolved

At spawn, highest first:

1. the profile named on the agent instance,
2. the model/provider recorded on the instance when the workspace was created,
3. `WSX_<AGENT>_MODEL` / `WSX_<AGENT>_PROVIDER` in the spawning process's
   environment.

A profile outranks a recorded model because it is the more deliberate choice: it
names an endpoint someone configured, where the recorded value is whatever
happened to be exported in the shell that created the workspace. A profile that
sets `base_url` but no `model` still defers to the recorded model — "same model,
different machine" is a normal thing to want.

Profiles are stored **by name**, so editing one updates every workspace pinned
to it. A name that no longer resolves is not fatal: the workspace opens on
ambient defaults with a warning, rather than becoming unopenable because of an
unrelated config edit.

## Parallel agents share one GPU

wsx is built to run many agents at once, and a local endpoint is the one case
where that stops being free. Several workspaces pointed at the same local server
do not run in parallel — they queue on one GPU and divide its context budget
between them. Two local workspaces and a third on a hosted endpoint will often
finish sooner than three local ones.
