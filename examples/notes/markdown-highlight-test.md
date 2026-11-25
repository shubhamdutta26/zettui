---
title: Markdown highlighting test
tags: [zettui, test]
---

# Heading one
## Heading two
### Heading three
#### Heading four
##### Heading five
###### Heading six

**Bold**, *italic*, ***bold italic***, ~~strikethrough~~, ==highlight==.
**Outer bold and *inner italic***.
*Outer italic and **inner bold***.
**Bold with `inline code` and [[Wiki note]]**.
β **Unicode** — café — 中文.
foo_bar_baz and \*escaped\* and unclosed **text

- [ ] Pending
- [x] Complete
* Unordered item
1. Ordered item
> A quote with **bold** and *italic*.

[[Antibody#Affinity]] #research #antibody/engineering
[@jain2017; @other2026] [-@jain2017] [^1]
[^1]: Example footnote.
[Website](https://example.org/a(b)c)
![Image](assets/image.png)
<https://example.org>

Inline math: $K_D = k_d/k_a$. Prices: $5 and $10.

$$
K_D = \frac{k_d}{k_a}
$$

| Clone | KD |
| --- | --- |
| VHH-10 | 15 nM |

Plain text a | b.

<!-- **not bold**
# Not a heading
-->

`# not a heading` and ``a ` b``.

```r
# Existing code highlighting
KD <- 1e-8
```

---

# Heading after code and comments
