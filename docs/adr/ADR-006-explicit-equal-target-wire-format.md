# ADR-006: Explicit equal-target wire format

## Status

Accepted

## Context

`EqualTarget::LineLength` and `EqualTarget::Radius` both contain an
`EntityId`. The former untagged serializer therefore wrote both variants as a
bare entity ID string. Deserializing that string could only select the first
variant, silently changing a radius target into a line-length target after a
round trip.

## Decision

1. Serialize line-length targets as `{ "line": "ent:..." }`.
2. Serialize radius targets as `{ "radius": "ent:..." }`.
3. Continue accepting legacy bare entity ID strings, interpreting them as
   `LineLength` because that is the only unambiguous compatibility behavior.
4. Reject objects that contain neither or both target keys.
5. Keep the `opencad.document.v0.1` schema/version. This is a compatible
   reader and canonical-writer correction; no migration branch is needed.
6. Permit both the legacy string and explicit object forms in the document
   JSON schema while documenting the explicit form as canonical.

## Consequences

- Radius equal constraints retain their kind through serialization and file
  round trips.
- Existing `.ocad` documents remain readable and are canonicalized on write.
- Golden expanded-directory fixtures change only where their serialized equal
  targets and checksums change; the schema version does not change.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Keep untagged strings | Cannot distinguish line lengths from radii |
| Add a schema version solely for this fix | Breaks otherwise compatible readers and adds no migration value |
| Encode a synthetic ID prefix | Pollutes semantic IDs and requires another parsing convention |
