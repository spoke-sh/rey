# `@rey/explorer`

`@rey/explorer` is Rey's reusable browser canvas and accelerated rendering
package. It owns the declarative React Three Fiber scene, bounded Three.js
WebGPU/WebGL2 lifecycle, terrain upload parity and allocation limits, semantic
globe fabric, and renderer diagnostics.

The package accepts already admitted, application-compiled scene inputs. It
does not fetch evidence, interpret workload documents, choose semantic level of
detail, mutate runtime state, or qualify what it renders. Those responsibilities
remain in `@rey/agent`; the deterministic accessible reference renderer also
remains there and imports only the package's pure globe-fabric subpath.

The public package entry exports `ExplorerCanvas`, its typed content/report
contract, the pure globe and terrain GPU compilers, renderer revisions, and the
structural scene input types. `@rey/agent` supplies its StyleX classes through
the canvas `className` and `readyClassName` props, so this package does not own
application layout or presentation tokens.
