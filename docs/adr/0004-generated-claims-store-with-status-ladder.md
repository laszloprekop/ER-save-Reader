# The claims store is pipeline-generated with a three-rung status ladder

The claims store (successor of ground_truth_offsets.json) is exclusively pipeline output:
hand-written inputs are limited to the evidence catalog and hypotheses, and regenerating
from scratch must reproduce the store byte-for-byte. Hand-editing the store is forbidden —
that is how convention-ambiguous entries accumulated last time. Claims carry a status:
hypothesis → corroborated (one verification method, e.g. multi-slot differential) →
verified (a kill transition, or two independent methods). Disproven claims remain as
tombstones so refuted ideas cannot be re-proposed (e.g. the (18,0)=43487 stride guess).
Applications (UI, elden-map) consume corroborated and verified claims only; anything else
renders as "unknown". Migration starts with anchor conformance (ADR-0003) because every
re-verification depends on a trustworthy anchor.
