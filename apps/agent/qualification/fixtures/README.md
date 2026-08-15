# Landscape Qualification Inputs

These native inputs are source-controlled candidates for separately admitted
Plan 0005 browser voyages. They are not retained evidence and the browser
harness never injects them into a scene.

`explicit-holes-terrain.geojson` is a 3×3 OGC CRS84 elevation grid with eight
valid vertices and one explicit center no-data vertex. A qualification workspace
registers it with role `terrain`, stages and commits the editor scene, qualifies
the repository's `scene-admission` workload, runs that workload against the
exact `SCENE@n`, and starts `rey agent` over the resulting state. The voyage may
then select `--landscape-workload explicit-holes`.

Generated screenshots and manifests remain beneath ignored `.rey` state. This
directory retains only reproducible native inputs, not proof artifacts.

One isolated qualification workspace can reproduce the admitted scene with:

```sh
mkdir -p "$QUALIFICATION_WORKSPACE/sys"
cp -R sys/scene-admission \
  "$QUALIFICATION_WORKSPACE/sys/scene-admission"
cp apps/agent/qualification/fixtures/explicit-holes-terrain.geojson \
  "$QUALIFICATION_WORKSPACE/explicit-holes-terrain.geojson"
rey editor generate terrain terrain.geojson \
  --workspace "$QUALIFICATION_WORKSPACE" \
  --id regional-controls --scene-id explicit-holes --seed 17 \
  --west -122.75 --south 37.25 --east -122.25 --north 37.75 \
  --features 2 --vertices 5
rey editor source add explicit-holes-terrain.geojson \
  --workspace "$QUALIFICATION_WORKSPACE" \
  --id explicit-holes-terrain --role terrain
rey editor add --workspace "$QUALIFICATION_WORKSPACE"
rey editor commit --workspace "$QUALIFICATION_WORKSPACE" \
  -m "Freeze explicit holes Landscape fixture"
rey workloads add --workspace "$QUALIFICATION_WORKSPACE"
rey workloads test --workspace "$QUALIFICATION_WORKSPACE" \
  --staged scene-admission -vv
rey workloads commit --workspace "$QUALIFICATION_WORKSPACE" \
  -m "Qualify scene admission"
rey workloads run --workspace "$QUALIFICATION_WORKSPACE" \
  scene-admission --scene SCENE@1
```

The workspace must contain the repository's exact `sys/scene-admission`
package before `workloads add`. `$QUALIFICATION_WORKSPACE` is an example task
variable, not a variable interpreted by Rey.
