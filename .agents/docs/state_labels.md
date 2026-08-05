# State Labels

## Architect Labels (`arch:`)
| Label | Color | Description |
|-------|-------|-------------|
| `arch:idle` | CCCCCC | No active architectural work |
| `arch:assigned` | E3F2FD | Work request assigned, analysis needed |
| `arch:analyzing` | BBDEFB | Analyzing requirements and constraints |
| `arch:waiting_for_management_clarification` | FFF176 | Waiting for product decisions |
| `arch:specifying` | C8E6C9 | Creating/updating technical spec |
| `arch:waiting_for_implementation` | FFF9C4 | Spec handed to Developer |
| `arch:waiting_for_developer_feedback` | FFE0B2 | Waiting for implementation questions |
| `arch:reviewing_implementation` | E1BEE7 | Evaluating completed implementation |
| `arch:completed` | DCEDC8 | Architectural work complete |
| `arch:blocked` | EF9A9A | Cannot continue, missing info |

## Developer Labels (`dev:`)
| Label | Color | Description |
|-------|-------|-------------|
| `dev:idle` | CCCCCC | No active implementation |
| `dev:assigned` | F3E5F5 | Spec assigned, preparing work |
| `dev:analyzing` | E1BEE7 | Analyzing spec and dependencies |
| `dev:implementing` | CE93D8 | Coding changes |
| `dev:waiting_for_architect_clarification` | FFF59D | Need architect input |
| `dev:waiting_for_management_decision` | FFE082 | Need product decision |
| `dev:validating` | C8E6C9 | Running tests/validation |
| `dev:ready_for_review` | A5D6A7 | Ready for Code Reviewer |
| `dev:addressing_review_feedback` | FFCC80 | Fixing review issues |
| `dev:waiting_for_review_approval` | FFE0B2 | Waiting for reviewer |
| `dev:preparing_delivery` | 80CBC4 | Preparing merge |
| `dev:delivering` | 4DB6AC | Pushing/merging code |
| `dev:completed` | 81C784 | Implementation delivered |
| `dev:blocked` | EF9A9A | Cannot continue |

## Reviewer Labels (`rev:`)
| Label | Color | Description |
|-------|-------|-------------|
| `rev:idle` | CCCCCC | No active review |
| `rev:assigned` | FFF3E0 | Implementation assigned for review |
| `rev:reviewing` | FFE0B2 | Actively evaluating |
| `rev:requesting_developer_changes` | FFCCBC | RFC issued, waiting fixes |
| `rev:waiting_for_developer_changes` | FFAB91 | Waiting for developer to fix |
| `rev:approved` | C8E6C9 | Implementation passes |
| `rev:handoff_to_developer` | A5D6A7 | Approval result returned |
| `rev:completed` | 81C784 | Review complete |
| `rev:blocked` | EF9A9A | Cannot continue |

## Handoff Labels
| Label | Color | Description |
|-------|-------|-------------|
| `handoff:pending` | 2196F3 | Handoff initiated |
| `handoff:accepted` | 4CAF50 | Handoff accepted |
| `handoff:rejected` | F44336 | Handoff rejected |

**Total: 33 labels**