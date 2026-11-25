---
title: Antibody Humanization — β
id: 202611251730
date: 2026-11-25
updated: 2026-11-25T17:30:00-05:00
published: false
reviewed: TRUE
missing: null
empty: ~
rating: 4.5
kd: 1e-8
negative: -12
hex: 0xFF
quoted_number: "123"
quoted_boolean: 'false'
quoted_key: "contains: colon # and hash"
"special:key": value
url: https://example.org/notes#section
sample: VHH 123 # actual comment
aliases: [VHH, Nanobody, "single domain"]
values: [true, null, -1.5, 1e-8]
metadata: {species: rhesus, active: true, count: 3}
tags:
  - antibody
  - humanization
  - research
clones:
  - name: VHH-10
    kd: 15
  - name: VHH-13
    kd: 9
settings: &defaults
  enabled: true
backup: *defaults
custom: !!str 123
summary: |-
  This is literal text.
  # This is not a YAML comment.
  **This is not Markdown bold.**
  ---
  date: not a nested key
notes: >
  This is a folded string.
  It continues on another line.
multiline: "First line
  # Still inside the string"
flow:
  [alpha,
   beta, 42]
# Actual YAML comment
status: complete
---

# Markdown resumes here

**Bold**, *italic*, [[Wiki note]], #research.

---

```yaml
# Existing fenced-code highlighting remains unchanged
name: example
```
