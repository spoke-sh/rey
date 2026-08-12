# ADR 0046: Read-First Scene Editor And Admission Packages

- Status: Accepted
- Date: 2026-08-11
- Extended by: [ADR 0055](0055-editor-project-state-ownership.md)
- Extends: [ADR 0041](0041-continuous-coordinate-topography.md) and [ADR
  0044](0044-explorer-projection-engine.md)
- Extended by: [ADR
  0056](0056-continuous-globe-mercator-county-grammar.md), which makes admitted
  editor scenes the primary semantic-Mercator and isometric-County fabric

## Context

Explorer is becoming a high-fidelity engine for projecting admitted,
high-dimensional evidence, but its inputs currently begin at one built-in
survey workload and immediately become a `rey.topography-patch.v1`. That skips
an important authoring and interchange plane. Surveys, humans, and agents need
to assemble detailed terrain sources, features, boundaries, hydrology,
markers, labels, and presentation hints before asking Rey to admit them.

This plane resembles a level editor, while `/explore` resembles its read-first
runtime projection. Treating the browser as the editor would violate the
read-only navigation boundary. Letting an editor write topography patches or
projection packets directly would bypass workload qualification, confuse
authored candidates with observed evidence, and make rendering state an
authority. Flattening standard geospatial artifacts into a Rey-only table
would also discard native geometry, raster, CRS, metadata, and relation
semantics.

GeoJSON, OpenStreetMap-derived exports, GeoPackage, GeoTIFF/Cloud Optimized
GeoTIFF, and typed Arrow artifacts offer useful conventions, but they do not
all carry the same meaning. In particular, RFC 7946 GeoJSON is geographic OGC
CRS84 longitude/latitude. It must not be relabeled as an arbitrary semantic
embedding. ADR 0047 separately defines explicitly synthetic
`semantic_longitude`/`semantic_latitude` axes with no Earth CRS; those layout
coordinates cannot replace a native GeoJSON CRS. OSM ways are not
automatically Rey paths, and a line in a survey
export is not proof that a road, river, dependency, or traversable passage was
discovered.

## Decision

Rey introduces a separate **scene editor candidate plane** owned by the `rey
editor` CLI. Generation is the single authoring entry point; agent editing of
the generated native artifacts is an intentional part of the loop. Its state
transition is:

```text
explicit bounds + generator hyperparameters
                    │
                    │ rey editor generate terrain <output> <parameters>
                    ▼
     rey.editor-project.v1 + generated native source
                    │
                    │ agent fine-tunes native WORKING bytes
                    ▼
             WORKING scene candidate
                    │ rey editor add
                    ▼
       INDEX + frozen native objects
                    │ rey editor commit -m <message>
                    ▼
       SCENE@n + immutable rey.scene-package.v1
                    │
                    └─ rey.scene-admission-request.v1
                                  │
                                  ▼ future explicit qualified workload
                   admitted topography/features
                                  │
                                  ▼
                    rey.projection-packet.v2
                                  │
                                  ▼
                              /explore
```

`HEAD → INDEX → WORKING` is the editor comparison model. HEAD means the latest
immutable `SCENE@n` editor commit; it does not mean the associated package is
admitted. INDEX contains the exact verified project snapshot and
content-addressed native bytes selected by `rey editor add`. WORKING is
re-observed from the project and its declared sources. `rey editor commit`
revalidates only INDEX and its frozen objects. On success it records a message,
timestamp, sequence, parent, immutable package, validation evidence, and
directed change set, then emits a candidate-only admission request. On failure
it does not advance HEAD. It does not re-observe mutable sources, execute a
survey, admit evidence, or change `/explore`.

The first executable command surface is:

```text
rey editor generate terrain <output.geojson> --id <source> --seed <seed> \
  [--scene-id <project>] \
  --west <lon> --south <lat> --east <lon> --north <lat> [hyperparameters]
rey editor status
rey editor add
rey editor commit -m <message>
rey editor log [-p] [-n <count>]
rey editor diff [--staged]
```

Every command has a human evidence rendering and structured JSON. ADR 0055
places the project declaration in the selected local editor store; native
source inputs remain workspace-contained bounded regular files, and symlinks
and path escapes are rejected. The `.rey/editor` store never replaces the
user-authored native source files.
The human `status` rendering follows the same Git-shaped operator grammar as
`rey env status`: it identifies `SCENE@n`, separates changes staged for commit
from changes still in WORKING, lists their semantic scene objects, and ends
with actionable state guidance. The successful `commit` receipt exposes the
frozen snapshot's validation evidence; retained messages, parents, packages,
and exact deltas belong to `log`; structured `status` retains the complete
typed state. There is no separate public `init`, `import`, or `validate`
command: `generate` bootstraps the project, agents fine-tune its native output,
and validation is a commit gate rather than an optional diagnostic ritual.

`generate terrain` is a deterministic authoring operation over explicit CRS84
bounds. Its effective seed, feature count, polygon resolution, scale interval,
uplift ratio, strength and variation, roughness and variation, anisotropy,
orientation and variation, edge jitter, and falloff are all validated and
embedded in a `rey.scene-generation.v1` foreign member of the generated native
GeoJSON. If the project is absent, generation creates it and uses
`--scene-id`, or the source ID by default, as its stable identity. Repeating the
same recipe yields the same generated base bytes. Changing a parameter changes
WORKING and is reviewed, staged, and committed through the ordinary editor
loop. Agents may subsequently edit the generated native artifact; the commit
binds those exact bytes and their semantic delta without pretending the base
recipe reproduces the manual edits. The command refuses to overwrite a file
that does not already carry matching generator ownership. Generated effects
remain authored candidate hints; they are not surveyed terrain or admitted
semantic truth.

The first source adapter accepts RFC 7946 GeoJSON `Feature` and
`FeatureCollection` documents with Point, MultiPoint, LineString,
MultiLineString, Polygon, MultiPolygon, and GeometryCollection geometry. It
requires stable string or number feature IDs, preserves the exact native bytes,
records an exact content digest, validates finite CRS84 positions and bounds,
and builds only a bounded feature index. Custom `crs` members are rejected.
The project fixes the coordinate system to `OGC:CRS84` while this is the only
adapter.

GeoJSON sources declare one explicit role: general features, markers, terrain
control, hydrology, or boundary. Marker points may carry bounded `title` or
`name`, `category`, `symbol`, `min_zoom`, `max_zoom`, and
`collision_priority` properties. These values are candidate label/LOD hints,
not observed importance or admission authority. Feature properties remain in
the native object and receive an exact digest; the index does not pretend to be
the native artifact.

The scene commit and its package retain:

- exact commit sequence, message, timestamp, parent, project, snapshot,
  package, generation recipe, and native-object identities;
- coordinate system, geographic bounds, source formats and roles;
- bounded feature and POI indexes with geometry kinds and coordinate counts;
- coverage, completeness, omissions, and effective limits;
- a directed parent-commit to commit change set; and
- a candidate-only authority marker plus a separate content-identified
  admission request.

The admission workload is a distinct future implementation slice. It must
verify the package and native-object identities, qualify the relevant source
adapters, bind coordinate/projection semantics, retain limitations and
lineage, and emit admitted terrain/feature evidence before `/explore` can read
anything from the package. A future editor preview must use a separate route or
explicit visual state that says `UNADMITTED`; it may never masquerade as the
admitted `/explore` world.

Detailed raster terrain and non-geographic semantic charts remain source
adapter work, not fields silently packed into GeoJSON. Planned adapter families
include GeoPackage for containerized vector/raster layers, GeoTIFF/COG for
georeferenced elevation and masks, Arrow for genuinely typed feature
attributes, and a Rey-native multiresolution terrain-field manifest for
provider-qualified semantic coordinate charts. Each requires an explicit
media/schema contract, CRS or semantic coordinate binding, validity and no-data
semantics, units, tiling, limits, and qualification before it is advertised.

## Consequences

- `/explore` remains a read-only projection of admitted evidence; it is not an
  authoring or admission UI.
- Agents and surveys gain a deterministic CLI surface for assembling and
  reviewing scene candidates without bypassing workload policy.
- Exact native artifacts survive commit packaging, while bounded indexes make
  status, diffs, history, POIs, and later admission inspectable.
- Agents can sweep deterministic terrain-control hyperparameters while every
  generated effect remains reproducible, diffable, and explicitly candidate
  only.
- GeoJSON interoperability is real but narrow: geographic CRS84 vectors and
  markers are implemented; GeoPackage, GeoTIFF/COG, Arrow, semantic charts,
  and raster terrain are not yet accepted.
- Lines retain their declared feature role. They do not become roads, rivers,
  source edges, or constructed/discovered paths by visual analogy.
- Scene commit creation is incomplete enabling work until one qualified admission
  workload exposes the resulting admitted inputs, results, deltas, omissions,
  limits, and lineage through the CLI and `/explore` consumes that same retained
  evidence.

## References

- [RFC 7946: The GeoJSON Format](https://www.rfc-editor.org/rfc/rfc7946)
- [OpenStreetMap elements](https://wiki.openstreetmap.org/wiki/Elements)
- [OGC GeoPackage](https://www.geopackage.org/)
- [OGC GeoTIFF](https://www.ogc.org/standards/geotiff/)
