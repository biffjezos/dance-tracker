# ACTIVE AGENT WORK

## shuffle-agent

STATUS:
IN PROGRESS

TASK:
Implement SHUFFLE operation as a normal graph operation with proper channel routing.

OWNERSHIP:
The following files are reserved and must not be modified by other agents:

- engine/src/operations/transform/shuffle.rs
- engine/src/operations/transform/mod.rs
- engine/src/compositor/metadata.rs

RULES:
- Do not edit other operation files
- Do not modify the inventory system
- Do not change the operation registration mechanism
- Follow existing patterns from other operations

DEPENDENCY CONTRACT:
When complete, this agent will provide:

- A working Shuffle operation that routes RGBA channels
- Proper serialization using "R", "G", "B", "A", "OFF" format
- Correct parameter names (red_channel, green_channel, blue_channel, alpha_channel)
- Color category for the operation
- Integration with existing operation architecture

Other agents should treat the Shuffle operation as a normal graph operation.

---
