# Graph Rebuilding Optimization

## Problem
The original `rebuildGraph()` function in `scripts/engine/graph.js` was called on every settings change and would:
- Recreate ALL nodes from scratch
- Not reuse existing nodes
- Cause performance issues with many nodes
- Orphan old WASM nodes without cleanup

## Solution
Implemented an **incremental update system** that:

### 1. Node ID Tracking
- Added `layerNodeIds` Map to track WASM node IDs for each layer
- Added `layerSettingsHash` Map to detect when settings actually change
- Each layer maintains its node ID across rebuilds when possible

### 2. Change Detection
- Created `hashSettings()` function to create stable hashes from settings objects
- Created `settingsChanged()` function to compare current vs previous settings
- Only rebuild nodes when their settings have actually changed

### 3. Incremental Building
- Modified each build function to check for existing nodes
- Reuse existing WASM nodes when settings haven't changed
- Track both primitive nodes and composite/wired nodes separately

### 4. Cleanup
- Clean up stale entries for removed layers
- Ready for when Rust/WASM node removal is implemented (generation mechanism already exists in `graph.rs`)

## Key Changes in `scripts/engine/graph.js`

### New Tracking Variables
```javascript
const layerNodeIds = new Map();      // layerId -> wasmNodeId
const layerSettingsHash = new Map(); // layerId -> hash for change detection
```

### New Helper Functions
```javascript
function hashSettings(settings)      // Create hash from settings
function getLayerNodeId(layerId)     // Get cached node ID
function setLayerNodeId(layerId, nodeId) // Cache node ID
function settingsChanged(layerId, settings) // Detect changes
function buildLayerNode(entry)       // Incremental node builder
```

### Modified Build Process
Each pass now:
1. Checks if a node already exists for this layer
2. Checks if settings have changed
3. Reuses existing node if unchanged
4. Creates new node only when necessary
5. Tracks the new node ID for future rebuilds

### Backward Compatibility
- Maintained all existing exports (`cachedContentIds`, `cachedWiredIds`, etc.)
- Existing callers of `rebuildGraph()` work without changes
- Output node ID resolution now checks both cached and tracked IDs

## Performance Impact

### Before
- N layers × M settings changes = N × M node creations
- All nodes recreated on every change, regardless of what changed

### After
- Only changed layers trigger node recreation
- Unchanged layers reuse existing WASM nodes
- Typical case: 1-2 node recreations per settings change instead of N

## Future Improvements

When the Rust/WASM core implements node removal:
1. The generation mechanism in `graph.rs` will automatically handle stale IDs
2. We can add explicit cleanup calls to remove orphaned nodes
3. Memory usage will be further optimized

## Testing

The optimization maintains the same external behavior:
- All existing tests should pass
- Output is identical to before
- Only performance characteristics have changed

## Files Modified
- `scripts/engine/graph.js` - Complete rewrite with incremental update system

## Files Created
- `GRAPH_OPTIMIZATION.md` - This documentation
