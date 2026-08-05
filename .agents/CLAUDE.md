---
title: CLAUDE.md
owner_role: "management"
version: 1
---

# Project

VFX Composer written in:

- Rust
- WebAssembly
- JavaScript
- HTML, CSS
- WGSL (WebGPU)

---

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

Roles are independent.

Each role has defined responsibilities and permissions.

---

# Global Rules

All agents shall:

- read relevant documentation before acting;
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

Changes require Management approval.

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

---

# Session Protocol

For every session:

1. Identify assigned role.
2. Read role instructions.
3. Read state definition.
4. Read working state.
5. Validate current state.
6. Resume assigned work.
7. Update working state after meaningful progress.

---

# File Access Rules

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

Each type of change must remain independently traceable.

---

# Completion Principle

A task is complete only when:

- intended behavior exists;
- implementation matches specification;
- validation is complete;
- required documentation is updated;
- responsible role state is updated.