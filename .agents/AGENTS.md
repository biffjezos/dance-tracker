<!-- .agents/AGENTS.md -->
---
title: AGENTS.md
note: "The Workflow in this folder is currently only information. Agents are asked to follow
these instructions, but it's not enforced. It is planned to implement a validating workflow engine. Fyou may use this in your workflow."
owner_role: "management"
version: 1
---
# Files + Directories

## Global (for all agents)

- Project Description:        .agents/docs/project_description.md
- Governance + Organization:  .agents/docs/governance_and_organization.md
- Guidelines + Conventions:   .agents/docs/guidelines_and_conventions/*
- Communication Protocol:     .agents/docs/workflow/communication_protocol.ms

Put advices, evaluations, implementation reports, RFC, RFI, and responses (answers) to requests into the respective sub-folders within this inter-agent communication-directory:

- Working Directory:          .agents/communication/

## Agent dependent

- Role Instructions:         .agents/roles/<role_name>/instructions/<role_name>.md
- State Definitions:         .agents/docs/state_definition_<role_name>.yaml
- Working State:             .agents/roles/<role_name>/state_<role_name>.yaml
- Role Directory:            .agents/roles/<role_name>/

# Multi-Agent Environment

This project is developed using multiple specialized AI roles.

The agent system consists of:

- Management
- Software Architect
- Software Developer
- Code Reviewer
- Technical Advisor

Each role has:

- role instructions;
- role directory;
- working state;
- state definition.

Roles have independent responsibilities and communicate only through the defined workflow and handoff process.

Each role has defined responsibilities and permissions.

---

# Global Rules

All agents shall:

- operate only after an explicit role assignment from Management or the user;
- read documentation and source files required for the assigned task;
- understand existing code before modifying it;
- follow approved specifications;
- follow role permissions;
- avoid assumptions when information is missing;
- request clarification when required.

Agents shall not:

- bypass assigned responsibilities;
- modify files outside their permissions;
- redefine requirements;
- silently change architecture.

---

# Role System

Each role operates according to:

role instructions
|
v
state definition
|
v
working state

## Role Instructions

Define:

- responsibility;
- permissions;
- procedures;
- restrictions.

Role instructions are immutable during normal operation.

Only Management may create, remove, or modify roles.

---

## State Definitions

State definitions define:

- allowed fields;
- allowed values;
- state transitions;
- validation rules.

Agents must not invent:

- new fields;
- new state values;
- new workflow states.

Only Management may approve state definition changes.
---

## Working States

Working states record the current position of each role.

Working states contain:

- active assignments;
- progress;
- blockers;
- waiting conditions;
- handoff information.

Working states must follow their state definition.

### Initial Working State

If a working state file does not exist for an assigned role:

- the role shall not invent workflow progress;
- the role shall create an initial working state only if permitted by its role permissions;
- the initial working state shall conform to the role state definition;
- the initial status shall represent that no active work is assigned.

If the role does not have permission to create its working state, it shall request Management initialization.
---

# Session Protocol

For every session:

1. Receive explicit role assignment.
2. Read:
   - role instructions;
   - applicable governance documents;
   - artifact format definitions;
   - state definition;
   - working state.
3. Validate current state.
4. Resume assigned work.
5. Produce artifacts according to the approved artifact formats.
6. Update working state after meaningful progress.

---

# File Access Rules

Agents may access only files permitted by their role permissions and required for their assigned task.

Agents may only modify files permitted by their role.

Role permissions are defined in the role instruction frontmatter.

Agents shall not modify:

- other role instructions;
- other role states;
- state definitions;
- architecture documents;

unless explicitly authorized.

---

# Repository Change Rules

Before changing code:

Agents must:

- understand current implementation;
- identify affected components;
- verify authorization.

Changes must be:

- traceable;
- scoped;
- reviewable.

Avoid:

- unrelated cleanup;
- speculative improvements;
- undocumented architecture changes.

# Change Management

Never mix:

- role/document changes;
- implementation changes;
- review changes.

Each change set shall contain only one category of work.

Architectural documentation, implementation, and review artifacts shall remain independently traceable.

---

# Completion Principle

A role task is complete only when:

- all responsibilities assigned to that role have been completed;
- required documentation owned by that role is updated;
- the role working state is updated.

Overall feature completion additionally requires:

- intended behavior exists;
- implementation matches specification;
- required validation is complete.

---