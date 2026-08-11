# ADR 0046: Read-First Scene Editor And Admission Packages

- Status: Accepted
- Date: 2026-08-11
- Extends: [ADR 0041](0041-continuous-coordinate-topography.md) and [ADR
  0044](0044-explorer-projection-engine.md)

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
embedding. OSM ways are not automatically Rey paths, and a line in a survey
export is not proof that a road, river, dependency, or traversable passage was
discovered.

## Decision

Rey introduces a separate **scene editor candidate plane** owned by the `rey
editor` CLI. Its state transition is:

```text
native survey artifacts + rey.editor-project.v1
                    │
                    ▼
             WORKING scene
                    │ rey editor add
                    ▼
       INDEX + frozen native objects
                    │ rey editor package
                    ▼
       immutable rey.scene-package.v1
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

`PACKAGE → INDEX → WORKING` is a comparison model, not a claim that PACKAGE is
admitted. PACKAGE means the latest immutable editor candidate. INDEX contains
the exact verified project snapshot and content-addressed native bytes selected
by `rey editor add`. WORKING is re-observed from the project and its declared
sources. `rey editor package` reads only INDEX, retains a directed change set,
and emits a candidate-only admission request. It does not re-observe mutable
sources, execute a survey, admit evidence, or change `/explore`.

The first executable command surface is:

```text
rey editor init --id <project>
rey editor import <source.geojson> --id <source> --role <role>
rey editor status
rey editor diff [--staged]
rey editor add
rey editor validate
rey editor package
rey editor inspect <exact-package-id>
```

Every command has a human evidence rendering and structured JSON. Project and
source inputs are workspace-contained bounded regular files; symlinks and path
escapes are rejected. The local `.rey/editor` store is a content-addressed
candidate/index cache and is never the sole copy of user-authored sources.

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

The scene package retains:

- exact project, snapshot, package, parent, and native-object identities;
- coordinate system, geographic bounds, source formats and roles;
- bounded feature and POI indexes with geometry kinds and coordinate counts;
- coverage, completeness, omissions, and effective limits;
- a directed prior-package to candidate change set; and
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
- Exact native artifacts survive packaging, while bounded indexes make status,
  diffs, POIs, and later admission inspectable.
- GeoJSON interoperability is real but narrow: geographic CRS84 vectors and
  markers are implemented; GeoPackage, GeoTIFF/COG, Arrow, semantic charts,
  and raster terrain are not yet accepted.
- Lines retain their declared feature role. They do not become roads, rivers,
  source edges, or constructed/discovered paths by visual analogy.
- Package creation is incomplete enabling work until one qualified admission
  workload exposes the resulting admitted inputs, results, deltas, omissions,
  limits, and lineage through the CLI and `/explore` consumes that same retained
  evidence.

## References

- [RFC 7946: The GeoJSON Format](https://www.rfc-editor.org/rfc/rfc7946)
- [OpenStreetMap elements](https://wiki.openstreetmap.org/wiki/Elements)
- [OGC GeoPackage](https://www.geopackage.org/)
- [OGC GeoTIFF](https://www.ogc.org/standards/geotiff/)
