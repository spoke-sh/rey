# ADR 0020: Environment Mapping Graph And Human CLI

- Status: Accepted
- Date: 2026-08-08
- Extends: [ADR 0019](0019-git-shaped-environment-history.md)
- CLI revision loop superseded by: [ADR 0021](0021-environment-admission-index.md)
- Default human status and diff projections superseded by:
  [ADR 0027](0027-environment-operator-delta.md) and
  [ADR 0028](0028-environment-three-plane-diff.md)
- Default human log projection superseded by:
  [ADR 0029](0029-environment-history-projection.md)

## Context

ADR 0019 establishes Git-shaped local environment revisions, but the CLI still
exposes manual certificate and bundle plumbing and retains a file-pair `diff`.
Those commands address proof implementation rather than the primary CLI user:
a programmer or agent operator trying to understand what the current runtime
environment contains and how it changed.

The built-in capability inventory is also too implicit. Rey needs an explicit,
reviewable graph of environment variables, input files, executables, and the
relationships that make them relevant. Agents should be able to generate and
revise that graph, while deterministic Rey parsing and observation decide what
is valid and what evidence actually exists.

## Decision

### Human Command Surface

The human environment loop accepted by this decision was:

```text
rey env status
rey env diff
rey env commit -m MESSAGE
rey env log [-p] [-n COUNT]
rey env inspect
```

`rey env diff` takes no snapshot-file operands. It performs a fresh bounded
observation and renders the authoritative `HEAD → WORKING` capability delta.
Before the first commit, the source is `EMPTY`. Explicit JSON emits
`rey.environment-diff.v1`. `status` remains the compact summary and does not
expand the full patch; `diff` and `log -p` are the patch surfaces.

ADR 0021 supersedes this revision loop with `status`, `add`, index-only
`commit`, unstaged `diff`, and `diff --staged`; it also removes `inspect` after
moving complete inventory and mapping evidence into `status`.

`prove`, `verify`, and `verify-bundle` are removed from the CLI. Their lower-
level proof and retention contracts remain available to workload evaluation,
internal composition, and tests. A user should encounter scoped proof through
workload results and retained evidence, not by manually wiring snapshot and
certificate files.

### Mapping DSL

The first mapping document is a checked-in YAML file, `rey.env.yaml` by
convention or an explicit workspace-relative `--map` path:

```yaml
schema: rey.env-map.v1
nodes:
  - id: cargo-home
    kind: variable
    name: CARGO_HOME
    sensitive: false
    capture: digest
  - id: workspace-manifest
    kind: file
    path: Cargo.toml
    required: true
  - id: cargo
    kind: executable
    name: cargo
    required: true
    potential_capabilities: [rust.build, rust.check, rust.test]
edges:
  - from: cargo-home
    to: cargo
    relation: configures
  - from: workspace-manifest
    to: cargo
    relation: input_to
```

The schema is `rey.env-map.v1`. Nodes are closed tagged variants:

- `variable` names one environment variable, whether it is sensitive, and a
  capture mode of `presence` or `digest`;
- `file` names one workspace-relative regular input and declares whether
  consumers should treat its absence as required admission evidence; and
- `executable` names one PATH-resolved executable plus sorted potential
  capability ids.

Edges contain exact `from`, `to`, and `relation` fields. Node ids and edge keys
are unique; every endpoint resolves; fields, counts, bytes, paths, and strings
are bounded. Node and edge order is canonicalized before semantic hashing.
The implementation uses `serde-saphyr` 1.x with only deserialization enabled;
the library does not define semantic identity, canonical output, merge rules,
includes, properties, or authority beyond this closed Rey schema.

The document is a graph declaration, not authority. An executable node is
resolved and metadata-inspected but never invoked by this provider. Potential
capabilities are navigation evidence and remain unadmitted until a separate
versioned adapter freezes implementation, arguments, effects, limits, and
guarantees. An edge records declared relevance; it does not prove a parser-
discovered dependency.

### Observation And Secrecy

The mapping provider projects its graph into the existing typed capability
snapshot so environment commits, status, diff, and log automatically bind it.
It emits one aggregate graph row with exact source, canonical graph, and limit
provenance, plus one exact observation row per node and declaration row per
edge. The enclosing snapshot binds all rows together.

Raw environment-variable values are never retained. Sensitive variables must
use `presence`; even a digest is rejected because it can become a stable secret
fingerprint. Non-sensitive variables may opt into a domain-separated value
digest. File observations retain path, regular-file status, byte length, and
content digest but not file bytes. Executables retain the declared logical name
and exact resolved path without executing it. Missing nodes remain explicit
unavailable rows; malformed YAML, unknown fields, invalid graphs, path escape,
symlinks, and exceeded graph/document/file bounds fail closed or become
explicit incomplete observation evidence according to the provider contract.

The aggregate row binds canonical graph identity, mapping-source identity, and
effective limits; node and edge rows bind their own canonical declarations and
observations. Adding, removing, or changing a variable, file, executable, edge,
observation, or bound therefore changes the enclosing environment snapshot and
appears in `env diff`.

## Consequences

- The normal environment workflow is coherent and Git-shaped.
- The environment snapshot describes why selected host surfaces matter, not
  only which built-in probes happened to run.
- Agents can propose YAML graph changes without acquiring execution authority
  or controlling evidence status.
- Proof libraries remain part of Rey without occupying the primary CLI persona.
- Dynamic path templates, file-content reference parsing, executable version
  invocation, schema migration, graph visualization beyond the terminal
  projection, workload admission from potential capabilities, and Spoke-backed
  graph retention remain later work.
