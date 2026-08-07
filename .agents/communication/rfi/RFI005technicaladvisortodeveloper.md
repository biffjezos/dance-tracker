RFI-ID: RFI-005
Created: 2026-08-07
Created-By: Technical Advisor
Target-Role: Software Developer
Related-Specification: none (workflow test)
Priority: Low
Status: Open

Subject: Workflow test — full Send/reply loop, fresh session

Context: All four roles are now on freshly-created sessions. This tests
the complete `Send` procedure end to end (see "Session Registry &
Delivery" in `communication_protocol.md`): a real artifact, pushed to a
branch, delivered via a verifiable pointer (not inline content), on a
session pair with matched `environment_id`.

Question: Please confirm receipt by sending a reply back to Technical
Advisor using the same `Send` procedure — commit your reply, look up
`role:technical_advisor`'s trigger in `.agents/session_registry/
technical_advisor.md`, and fire it with a pointer to your reply. No
special "back channel" — it's the same procedure, just aimed back at
the role that sent you this one.

Reason: Verifying the full round trip works on fresh sessions, not just
one-directional delivery.

Impact if unanswered: None structurally, but it means the round-trip
isn't confirmed working yet.
