# Vendored Hifi Kinetic Grammar

Rey vendors the minimum TypeScript source needed for `@hifi/core` and
`@hifi/kinetic` because the experimental packages are not yet published to
npm.

- Upstream: <https://github.com/rupurt/hifi>
- Revision: `0440cfe774405070facdb1106f3e247fa980060f`
- Package version: `0.1.0`
- License: MIT; see `LICENSE`

The files below are copied without semantic modification. Vite and TypeScript
aliases preserve the upstream package entry points. Update this revision,
license copy, and the UI server's reported grammar revision together.

At this revision, `KineticButton` and `KineticSurface` adopt Hifi's refined
directional press travel and layered edge, highlight, hard-shadow, and
soft-shadow lighting model. The other vendored core and Kinetic sources remain
byte-identical to the previous Rey pin. Rey's authored application styles
follow Hifi's StyleX architecture introduced by upstream commit `9a981c5`.
