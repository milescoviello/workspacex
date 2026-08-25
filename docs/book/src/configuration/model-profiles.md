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
| `auth_token_env` | **Name** of an environment variable holding the token |
| `max_context` | Context window to advertise, when it differs from the model's default. Must be greater than zero |

A profile must set at least one of `base_url` or `model`, or it would do
nothing. Blank lines and `#` comments are ignored.

Everything is checked when the setting is written rather than when an agent
spawns. A `base_url` with no scheme, or a `max_context` of zero, can only be a
mistake — and left to spawn time it surfaces as an opaque failure inside an
agent, long after the person who typed it has moved on.

## Which agents can use which fields

| Agent | `model` | `base_url` / `auth_token_env` / `max_context` |
| --- | --- | --- |
| `claude` | yes | **yes** |
| `pi`, `hermes`, `codex`, `omp` | yes | no |

Only Claude Code is wired to an arbitrary endpoint. The others accept a
profile's `model` but reach their endpoint through their own configuration, by
mechanisms wsx does not model — so pinning one of them to a profile that sets
`base_url` applies the model and nothing else.

That is allowed, because pinning a profile for its model alone is reasonable.
It is not silent: the CLI warns when you pin it, and wsx records no endpoint for
that session, so the dashboard never claims the workspace is on a server its
agent never contacted.

```console
$ wsx workspace create backend --agent codex --profile local-qwen
pinned to model profile local-qwen
warning: profile 'local-qwen' sets base_url, but codex reaches its endpoint
through its own config — only the model will be applied
```

## Choosing a profile

**In the TUI**, press `n` for a new workspace and `^p` to cycle the model, the
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

and the `model` detail-bar module says the same thing on its second line. To
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
