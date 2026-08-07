<!-- .agents/docs/governance_and_organization.md -->
---
title: Governance and Organization
path: ".agents/docs/governance_and_organization.md"
---
# Governance and Organization

## Authority Model

Management defines what must be achieved.
Management owns governance documents and role definitions.

Approved architecture documents and specifications define intended technical reality.

The repository represents current technical reality.

Agents must identify and resolve differences between intended technical reality and current technical reality.

# Artifact Governance

All generated documents must follow the approved artifact formats.

The artifact format definition is authoritative for:

- required fields;
- artifact structure;
- naming;
- document sections.

Role instructions may define when an artifact is created, but may not redefine its structure.

---

# Role Responsibilities

## Management

Owns:

- requirements;
- priorities;
- product decisions;
- acceptance of outcomes.

Provides:

- objectives;
- constraints;
- priorities.

### Management Checkpoint

After an implementation has been approved by the Code Reviewer and all roles are idle, Management may choose one of the following:

- assign the next work item;
- request an architectural review of the completed implementation.

An architectural review is advisory. It does not replace Code Review or reopen an approved implementation unless Management explicitly assigns follow-up work.
---

## Software Architect

Owns:

- system architecture;
- technical specifications;
- architecture decisions;
- design consistency.

Provides:

- specifications;
- ADRs;
- architecture guidelines.

Does not:

- implement production code.

---

## Software Developer

Owns:

- implementation of approved specifications;
- tests required by specifications;
- implementation reporting.

Does not:

- redefine architecture;
- change requirements.

---

## Code Reviewer

Owns:

- verification of implementation correctness;
- verification of specification compliance;
- quality evaluation.

Does not:

- redesign architecture;
- implement changes.

---

## Technical Advisor

Owns:

- technical investigation;
- analysis;
- recommendations.

Does not:

- make binding architectural decisions.

---

# Communication

Management may not always have knowledge of technical terminology.
Use appropriate non-technical English phrasing or ASD-STE100 when required.

## Communication Flow

Management <-> Software Architect <-> Software Developer <-> Code Reviewer

This chain is the default task-assignment and hand-off path.

RFI, RFC, and Evaluation exchanges, and a Handoff notifying the next
role that a piece of work is complete and ready for it (e.g. Software
Developer -> Code Reviewer after pushing an implementation — filing the
relevant artifact, such as an Implementation Report, with `Target-Role`
set accordingly), may be addressed directly to whichever role owns the
answer or is the intended receiver — not only the adjacent role in the
chain above (e.g. Software Architect <-> Code Reviewer is valid for a
specification question). Delivery uses the mechanism in
`communication_protocol.md`'s "Session Registry & Delivery" section.
Addressing a message to a role never grants that role's
file-modification permissions to anyone else — see each role's own
`can_modify`/`cannot_modify` list.

Technical Advisor:

Management <-> Technical Advisor
Technical Advisor <-> any role (RFI/RFC/Evaluation exchange only, not
task assignment or review) — broadened 2026-08-07 from Architect-only,
so every role can reach Technical Advisor for a consultation answer
without relaying through the Architect.

All consultations logged in working state.

---

# Document Ownership

| Document | Owner |
|---|---|
| Requirements | Management |
| Architecture | Software Architect |
| Specifications | Software Architect |
| Source Code | Software Developer |
| Tests | Software Developer |
| Review Results | Code Reviewer |
| Technical Analysis | Technical Advisor |

---

# Change Authority

Roles may modify only documents and files within their assigned ownership.

A role may provide feedback on another role's output but may not silently replace it.

---

# Reality Synchronization

The system always maintains two realities:

## Intended Reality

Defined by:

- requirements;
- specifications;
- architecture;
- ADRs;
- conventions.

## Current Reality

Defined by:

- repository contents;
- tests;
- runtime behavior.

The purpose of the workflow is to continuously reduce the difference between these two states.