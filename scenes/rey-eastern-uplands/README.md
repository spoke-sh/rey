# Rey Eastern Uplands

This source-controlled fictional region is a bounded multi-region Landscape
fixture. Its western grid edge copies the exact validity, centimeter elevation,
material, and first interior elevation slope from the separately authored Rey
County v15 eastern edge. A bounded corridor transitions from that exact edge
into a low-pass boundary trend before independent landforms enter through a
zero-slope envelope inside its explicit boundary. Neither source claims a
merge, gap fill, Earth elevation, or survey coverage.

Regenerate and verify the packed terrain with:

```sh
node scenes/rey-county/generate-terrain.mjs
node scenes/rey-eastern-uplands/generate-terrain.mjs
node scenes/rey-eastern-uplands/generate-terrain.mjs --check
pnpm --filter @rey/agent exec vitest run scripts/rey-regional-seam.test.mjs
```

Admission remains explicit:

```sh
rey editor source add scenes/rey-eastern-uplands/boundary.geojson \
  --id rey-eastern-uplands-boundary --role boundary \
  --scene-id rey-eastern-uplands
rey editor source add scenes/rey-eastern-uplands/terrain.geojson \
  --id rey-eastern-uplands-terrain --role terrain
rey editor add
rey editor diff --staged
rey editor commit -m "Admit Rey Eastern Uplands"
rey workloads run scene-admission --scene SCENE@n
```

Only the retained scene-admission results and server-owned regional composition
may qualify the shared boundary. Checkout or generator output alone is not
Explorer evidence.
