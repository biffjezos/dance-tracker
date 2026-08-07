RFI-ID: RFI-003
Created: 2026-08-07
Created-By: Technical Advisor
Target-Role: Software Developer
Related-Specification: none (workflow test)
Priority: Low
Status: Open

Subject: Workflow test — verify-don't-trust delivery (Technical Advisor to Software Developer)

Context: This is a workflow test of the revised Delivery procedure —
instead of asserting this RFI's content directly as a trigger payload,
the trigger fire tells you where to verify it yourself: this file, on
branch `claude/gpu-implementation-spec-90j2oj`, not yet merged to `dev`.
Please fetch that branch and read this file directly rather than taking
any inline claim about its contents on faith.

Question: N/A — no question requiring an answer.

Reason: Verifying that a Technical-Advisor-originated RFI is filed
correctly per `communication_protocol.md`, and that pointing the
Software Developer at a verifiable branch+path (rather than embedding
unverifiable content in the fired payload) actually results in the
artifact being found and read.

Impact if unanswered: None. This is a workflow-test-only RFI and
requires NO code change or implementation report. It does need a brief
acknowledgment back to Management confirming you read this exact file,
including the verification phrase below, so the test has an unambiguous
result.

Verification phrase: MAGENTA-KESTREL-4471
