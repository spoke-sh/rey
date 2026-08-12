# Rey County

Rey County is an authored **editor candidate** built from the first fresh v1
Rey environment and workload run. It exercises every GeoJSON role currently
accepted by `rey editor`: boundary, terrain control, hydrology, features, and
markers.

The geometry uses an arbitrary OGC CRS84 extent because GeoJSON admission
requires geographic longitude/latitude. It is not an Earth survey, a claim
that land exists at this extent, a semantic-distance map, or admitted
topography. District shapes, elevation controls, streams, and labels are
authoring choices whose properties cite the exact Rey evidence that motivated
them. Hydrology is a candidate natural feature and is not derived from the 76
survey source edges; no road, route, or traversability is claimed.

## Fresh evidence basis

- Environment: `ENV@1`, commit
  `blake3:6070a3c40b82ce885f912b522dfd9977d219412267a75dcf4de6b428bedce831`,
  capability snapshot
  `blake3:fc2e6648a63ba3df153b710b8616f0eff8602cd2f5fb2a41da0055a3549493d8`.
  Rey observed 14 capabilities; Git 2.55.0 and ripgrep 15.2.0 were identity
  probed, seven of eight declared applications were found, and Copilot was not
  found.
- Context survey: package
  `blake3:12ae63362e55d82f00367a5dea5eb75229c3de784d8072494b19d0fe007e39ff`,
  graph `blake3:27083b635f810fae7213484cfea07c5f1ca9bc11bd17e1ec4588398ce663497a`,
  run `blake3:73d579860d01e1d334204db433d875e8d730c1af2c259628786fec47f04c8369`,
  patch `blake3:2f4c0c28c1b5fb259c1b5b7c7252abd75947a89bd49ab50dfd5f66f81ebda448`,
  topography
  `blake3:c1d64839c0d95eb4886ecfd24759806bf72bf65391afe0a09aba429b5db5427f`,
  and projection packet
  `blake3:b767c2dd78fde8caa548436bb7372cdefd99da8f5d7c2df62dad0a94517aaa94`.
  Five seeds produced 56 anchors, 76 source edges excluded from terrain/path
  geometry, one frontier probe, and nine retained field channels.
- Label normalization: package
  `blake3:d1a67ec0d2a0e5849642690a576051fcd261e3b4b69ca68e16651773f2941426`,
  graph `blake3:88c196d992f65fe8d56be7f4d6a704afe4484a928c2dd578754dc9437014f7b5`,
  and run
  `blake3:70e6615ebcd02f121f506c5e0bb34389255a72d51019f5de846f28dc55922d7a`
  produced the canonical label `REY COUNTY`.

## Retention transcript

The checked-in native files are the authored source fixture. They include
generated terrain-control lineage plus exact agent fine-tuning and additional
feature families; the generator recipe is not presented as reproducing those
subsequent edits. The project declaration and `SCENE@1` history that originally
grouped these files are local Rey state and are not checked into the workspace.
These sources therefore do not become an ambient default scene when
`rey editor status` runs.

The historical retention loop was:

```text
rey editor status
rey editor diff
rey editor add
rey editor diff --staged
rey editor commit -m 'establish Rey County'
rey editor log -p
```

For a new scene, `rey editor generate terrain ... --scene-id <scene>` creates
`.rey/editor/project.json` and the first workspace-native source. The agent then
fine-tunes generated files in WORKING before using the same retention loop.
There is no separate initialization, import, or validation command; registering
an existing multi-source fixture remains future editor work. Commit validates
the frozen INDEX and refuses to advance on failure.

Packaging freezes a candidate only. `/explore` cannot consume this package
until a separate qualified scene-admission workload is implemented.
