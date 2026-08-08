RFC-ID: RFC_004_OPERATIONS_GHOST_DELEAY
Created: 2025-12-28T08:30
Created-By: management
Target-Role: software_architect
Related-Specification: unknown
Priority: medium
Status: Open

Severity: Low
Finding: Delay does not work as intended
Evidence: UI blackbox testing
Required Change:

The GHOST delay does not work as expected. There are two ways a GHOST can be rendered:

SPATIAL: The spatial X, Y settings work as intended. The distance for each ghost is computed from the previous ghost.
DELAY: Is the temporal equivalent of the spatial distance. A Delay of 1 represents a distance from the actual frame
to FRAME - DISTANCE. The ghost is rendered based on the mask of the delayed frame not the actual frame.

Spatial and DELAY are not mutually exclusive. The Spatial Distance is an additional distance to the temporal distance.

Example:
Spatial X = 0, Y = 0
GHOST COUNT = 1
DELAY = 0

would render the ghost exactly on the same position as the actual frame (basically a copy of the source frame).

DELAY = 1

shows the mask of the previous frame. in a sequence of frames two masks are visible, the source mask (or footage) and the copy of the previous frame.

GHOST COUNT = 5.

Only the first ghost is computed from the previous frame of the source. All subsequent ghosts compute a delay from the previous GHOST (or ghost layer).

SPATIAL X = 5, Y = 10

All ghost layers are shifted by X and Y. from their current position.

If there's no specification please backfill. Let the software developer fix it.

Acceptance Condition: Management Approval
