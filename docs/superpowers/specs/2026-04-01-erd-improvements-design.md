# ERD Diagram Improvements Design

## Goal

Polish the ERD diagram: theme-aware rendering, smarter node layout, cleaner edges, self-loop handling, and high-quality SVG export.

---

## 1. Node Redesign

### Header format
Display `schema.table_name` as a single string in the header. Remove the separate right-aligned schema label.

### Background
Replace `bg-card` with `bg-[--theme-bg-secondary]` on the node container — always an opaque surface colour from the active theme. The current `bg-card` resolves to transparent in some themes, making nodes unreadable.

### Auto-width
Compute node width in `erd-layout.ts` per table instead of the fixed `TABLE_NODE_WIDTH = 220`. Estimate using:
```
width = max(
  (schema + "." + table_name).length,
  max over columns of (column_name.length + raw_type.length + 10)
) * 7 + 80   /* 7px per char at text-xs, 80px for badges/padding */
```
Clamp between `200` and `500`. Set as the node's top-level `width` field — @xyflow uses this to size the node container. `ErdTableNode.svelte` removes the hardcoded `style="width: {TABLE_NODE_WIDTH}px"` and becomes `w-full` so it fills whatever width @xyflow assigns (including after NodeResizer drag).

### Handles — FK and PK columns only
Currently every column row has a source and target handle. Change to:
- Compute `fkColumns = new Set(table.foreign_keys.map(fk => fk.column_name))`
- Only render `<Handle>` elements on rows where `col.is_primary_key || fkColumns.has(col.column_name)`
- All other columns: no handles, no connection points

### Badges and icons
| Column type | Badge | Icon |
|-------------|-------|------|
| Primary key | Amber `PK` text | `IconKey` (tabler) |
| Foreign key | Blue `FK` text | `IconArrowUpRight` (tabler) |
| Both PK + FK | Both badges | Both icons |
| Neither | — | — |

### NodeResizer
Add `<NodeResizer minWidth={200} maxWidth={500} />` inside `ErdTableNode.svelte`. Dragging a corner handle resizes the width; height remains auto (number of columns × `COLUMN_ROW_HEIGHT`).

---

## 2. Background

Pass the live theme border colour to the `<Background />` component:
```svelte
<Background color={getComputedStyle(document.documentElement).getPropertyValue('--theme-border-subtle').trim()} />
```
This reads the CSS variable at render time so it updates when the user switches themes.

---

## 3. Edges

### Arrow direction
Add `markerEnd: { type: MarkerType.ArrowClosed }` to all edges in `erd-layout.ts`. Arrow points from FK column (source) → referenced PK column (target), i.e. the "references" direction.

### Normal FK edges (two distinct tables)
- Type: `smoothstep`
- Source handle: `${fk.column_name}-source` (right side of FK table)
- Target handle: `${fk.ref_column}-target` (left side of referenced table)
- Flow is strictly left → right matching the ELK `RIGHT` direction

### Self-referencing edges (source table === target table)
- Detected in `erd-layout.ts` when `sourceId === targetId`
- Edge type: `'selfLoop'` (custom)
- Source handle: `${fk.column_name}-source` (right side)
- Target handle: `${fk.ref_column}-source` (right side — note: both on the right so the loop never passes through the node)
- Custom component `ErdSelfLoopEdge.svelte`:
  - Receives `sourceX, sourceY, targetX, targetY` (both on the right edge of the node)
  - Draws SVG cubic bezier: exit right → bulge 60px to the right → arc back → re-enter at target y
  - Path: `M sourceX sourceY C (sourceX+60) sourceY, (targetX+60) targetY, targetX targetY`
  - Same `ArrowClosed` marker at the tip

---

## 4. Layout — ELK

### Replace dagre with elkjs
- Remove `@dagrejs/dagre` dependency
- Add `elkjs` + `@types/elkjs`
- `buildErdGraph` becomes `async`

### ELK configuration
```js
{
  'elk.algorithm': 'layered',
  'elk.direction': 'RIGHT',
  'elk.spacing.nodeNode': '80',
  'elk.layered.spacing.nodeNodeBetweenLayers': '150',
  'elk.spacing.edgeNode': '40',
  'elk.layered.nodePlacement.strategy': 'BRANDES_KOEPF',
}
```

### Node representation for ELK
Each node passed to ELK:
```js
{ id, width: computedWidth, height: TABLE_HEADER_HEIGHT + columns.length * COLUMN_ROW_HEIGHT }
```

### Edge representation for ELK
Self-loop edges are excluded from ELK layout input (ELK does not handle self-loops; their positions are derived from their node position at render time).

### ErdView.svelte async update
```svelte
$effect(() => {
  buildErdGraphWithSavedPositions(tables).then(result => {
    nodes = result.nodes;
    edges = result.edges;
  });
});
```
Same pattern for `autoLayout()`.

---

## 5. Export — SVG

### Switch from PNG to SVG
Replace `toPng` with `toSvg` from `html-to-image`. SVG is vector — infinitely scalable, arrowhead markers preserved natively, no pixel resolution concerns.

### Capture approach
Same as current: target `.svelte-flow__viewport`, compute transform via `getNodesBounds` + `getViewportForBounds`.

```ts
const dataUrl = await toSvg(viewport, {
  width: IMAGE_WIDTH,
  height: IMAGE_HEIGHT,
  style: {
    width: `${IMAGE_WIDTH}px`,
    height: `${IMAGE_HEIGHT}px`,
    transform: `translate(${transform.x}px, ${transform.y}px) scale(${transform.zoom})`,
  },
});
```

### JS → Rust handoff
`toSvg` returns a data URL: `data:image/svg+xml;charset=utf-8,<url-encoded-svg>`. Decode in JS before sending:
```ts
const dataUrl = await toSvg(viewport, { ... });
const svg = decodeURIComponent(dataUrl.replace(/^data:image\/svg\+xml;charset=utf-8,/, ''));
await invoke('save_svg_file', { path, svg });
```

### Rust side
Replace `save_png_file` with `save_svg_file`:
- Takes `path: String` and `svg: String` (plain text, no base64 needed)
- Calls `fs::write(path, svg.as_bytes())`
- Remove `base64` crate from `Cargo.toml`

### Save dialog
```ts
const path = await save({
  defaultPath: `erd-${schema}-${date}.svg`,
  filters: [{ name: 'SVG Image', extensions: ['svg'] }],
});
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/lib/components/erd/ErdTableNode.svelte` | Auto-width, opaque bg, FK/PK-only handles, FK badge + icons, NodeResizer, schema.table header |
| `src/lib/components/erd/ErdSelfLoopEdge.svelte` | New — custom self-loop SVG edge |
| `src/lib/components/erd/erd-layout.ts` | Replace dagre with ELK (async), auto-width, ArrowClosed markers, self-loop edge type, exclude self-loops from ELK input |
| `src/lib/components/erd/ErdView.svelte` | Async graph build, themed Background color, toSvg export, save_svg_file invoke |
| `src-tauri/src/commands/util_commands.rs` | Replace save_png_file with save_svg_file (plain text write, no base64) |
| `src-tauri/src/plugins/core.rs` | Replace save_png_file with save_svg_file in macro |
| `src-tauri/Cargo.toml` | Remove base64, no new deps |
| `package.json` / `pnpm-lock.yaml` | Remove @dagrejs/dagre, add elkjs + @types/elkjs |
