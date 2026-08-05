---
state_definition:
  name: state_definition_software_developer
  version: 1
  owner_role: management
description:
  purpose: >
    Defines the operational state model for the Software Developer role.
    The state records active implementation work, progress, validation,
    blockers, and handoff information.
fields:
  active_specification:
    required: true
    type: object
    properties:
      id:
        type: string
        required: true
        description: >
          Identifier of the approved specification currently being implemented.
      version:
        type: integer
        required: true
        description: >
          Version of the active specification.
  status:
    required: true
    type: enum
    values:
      idle:
        meaning: >
          No implementation assignment is currently active.
      assigned:
        meaning: >
          A specification has been assigned and implementation analysis
          has started.
      implementing:
        meaning: >
          Code changes are actively being performed.
      testing:
        meaning: >
          Implementation is complete and validation is being executed.
      blocked:
        meaning: >
          Implementation cannot continue due to unresolved dependencies,
          missing information, or external decisions.
      ready_for_review:
        meaning: >
          Implementation is complete and ready for Code Reviewer evaluation.
      completed:
        meaning: >
          Implementation and review workflow is complete.
  current_task:
    required: false
    type: object
    properties:
      name:
        type: string
        required: true
        description: >
          Short identifier of the current implementation task.
      description:
        type: string
        required: true
        description: >
          Detailed explanation of the current work being performed.
  progress:
    required: true
    type: object
    properties:
      completed:
        type: list
        required: true
        description: >
          Completed implementation phases or tasks.
      remaining:
        type: list
        required: true
        description: >
          Remaining implementation phases or tasks.
  modified_files:
    required: true
    type: list
    description: >
      Files modified during the current implementation task.
    item_type:
      type: string
  tests:
    required: true
    type: object
    properties:
      executed:
        type: list
        required: true
        description: >
          Tests or validation commands that have been executed.
      results:
        type: list
        required: true
        description: >
          Results of executed tests and validation.
  blockers:
    required: true
    type: list
    description: >
      Current issues preventing implementation progress.
    item_type:
      type: string
  rfis:
    required: true
    type: list
    description: >
      References to Requests For Information created during implementation.
    item_type:
      type: string
  handoff:
    required: true
    type: object
    properties:
      target_role:
        required: true
        type: enum
        values:
          null:
            meaning: >
              No handoff is currently planned.
          code_reviewer:
            meaning: >
              Implementation is being handed to Code Reviewer.
          software_architect:
            meaning: >
              Architectural clarification is required.
          management:
            meaning: >
              Management decision is required.
      status:
        required: true
        type: enum
        values:
          null:
            meaning: >
              No handoff exists.
          pending:
            meaning: >
              Handoff preparation has started.
          sent:
            meaning: >
              Handoff has been submitted.
          accepted:
            meaning: >
              Receiving role has accepted the handoff.
state_transitions:
  idle:
    allowed:
      - assigned
  assigned:
    allowed:
      - implementing
      - blocked
  implementing:
    allowed:
      - testing
      - blocked
  testing:
    allowed:
      - ready_for_review
      - implementing
      - blocked
  ready_for_review:
    allowed:
      - completed
      - implementing
  blocked:
    allowed:
      - assigned
      - implementing
  completed:
    allowed:
      - idle