---
title: CLAUDE.md
owner_role: "management"
token_count: 492
---
# Dance Tracker 5000

VFX Composer written in rust, wasm, js html, wgsl.

## Multi-Agent Environment

This project is being developed in a multi-role and multi-agent environment.
This document applies to all agents and agent roles.
Do not change this document without explicit permission.

## Do always

Read the code.
Understand it.
Ask questions if needed.
Don't guess.
Run tests.

## Your Role and role_directory

### Identifying your role

At the beginning of each session identify your role. If you have identified your role, state your role clearly in the chat. If you cannot infer your role within the
first few sentences exchanged in the conversation, ask the human session partner to assign a role.

Examples:

**Identification successful**
```
"Assigned role: Software Architect"
```
**Identification unsuccessful**
```
"Before we start, what's my assigned role today / in this session?"
```

### Role Directory (folder)

Each role has been assigned a 'role_directory' within:

```
.agents/roles/<YOUR ROLE>
```

You are the only agent-owner of this directory.

You can add, edit, remove, rearrange files and sub-folders in your role-directory.

Your files in your role-directory can be referenced and may be used by any other agent. Keep them concise, precise, correct, neutral and technical in language.

Keep all files up-to-date at any time. Amendments must be immediately committed, pushed and merged with the remote-dev branch.

Delete outdated or irrelevant files. Ask the human session partner if the benefit of keeping a file is unclear.

Never mix amendments to this folder (commits, pushes, merges) with any other work (planning, coding, reviewing).

## Instruction files

Once you have been assigned a role, read and follow the 'instructions_<YOUR_ROLE>.md' at the beginning or after compaction of a session.

## Access

### General

You are not allowed to change files outside of your role folder.

### Onwer Role

The 'owner_role' in the frontmatter of markdown files defines the agent's role that is allowed to change a file after permission by the human session partner.

## Instruction files

Do not edit instruction files that start with "instruction_" in any folder.
