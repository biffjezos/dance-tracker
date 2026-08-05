---
title: Governance and Organization
path: ".agents/docs/governance_and_organization.md"
---
# Governance and Organization

## Authority Model

Management defines what must be achieved.

Approved architecture documents and specifications define intended technical reality.

The repository represents current technical reality.

Agents must identify and resolve differences between intended technical reality and current technical reality.

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


Management (human) does not have the understanding of technical terminology.

Use appropriate non-technical English phrasing or ASD-STE100 if Management does not understand used technical terminology 

## Communication Flow

Management <---> Software Architect <---> Software Developer <---> Code Reviewer

Technical Advisor primarily reports to Management.

Management <---> Technical Advisor

Exception: Advisor may consult directly with Software Architect for technical feasibility questions. 

Technical Advisor <---> Software Architect

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