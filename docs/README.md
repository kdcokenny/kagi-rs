# Documentation Index

Use this index to find user onboarding, reference material, and maintainer/operator policy.

## Start here by role

| Role | First doc | Then |
|---|---|---|
| New user / evaluator | [`../README.md`](../README.md) | [`../mcp/README.md`](../mcp/README.md) or [`../sdk/README.md`](../sdk/README.md) |
| Rust integrator | [`../sdk/README.md`](../sdk/README.md) | [`endpoint-auth-version-matrix.md`](./endpoint-auth-version-matrix.md) |
| Agent host integrator | [`../mcp/README.md`](../mcp/README.md) | Fetch-pairing + verification sections in the same README |
| Contributor | [`../README.md`](../README.md) | Package README for the surface you are changing |
| Maintainer / operator | [`releasing.md`](./releasing.md) | Workflow files under `.github/workflows/` |

## User docs

| Doc | Audience | Purpose |
|---|---|---|
| [`../README.md`](../README.md) | Everyone | Workspace front door and project map |
| [`../sdk/README.md`](../sdk/README.md) | Rust developers | SDK install, surfaces, endpoint support |
| [`../mcp/README.md`](../mcp/README.md) | Agent builders | MCP setup, host integration, tool usage |
| [`../cli/README.md`](../cli/README.md) | CLI users/contributors | Planned status and direction |

## Reference docs

| Doc | Purpose |
|---|---|
| [`endpoint-auth-version-matrix.md`](./endpoint-auth-version-matrix.md) | Source of truth for endpoint/auth/version support in SDK v1 |

## Maintainer / operator docs

| Doc | Purpose |
|---|---|
| [`releasing.md`](./releasing.md) | Release tag policy, token posture, helper usage, workflows, live-test policy, protected environment contract |

## Next-step navigation

- Need endpoint/auth/version truth for a code change? → [`endpoint-auth-version-matrix.md`](./endpoint-auth-version-matrix.md)
- Need host setup commands and verification? → [`../mcp/README.md`](../mcp/README.md)
- Need SDK examples/config patterns? → [`../sdk/README.md`](../sdk/README.md)
- Need maintainer release/runbook policy? → [`releasing.md`](./releasing.md)
