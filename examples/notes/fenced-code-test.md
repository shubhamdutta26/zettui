# Fenced code highlighting

```{r, echo=false}
# R: scientific notation and functions
ka <- 1e5
kd <- 1e-3
KD <- kd / ka
print(KD)
if (TRUE) print(NA)
let <- 0xFF
```

```python
def affinity(ka, kd):
    """Multiline documentation
    # This is still a string
    """
    return kd / ka
print(affinity(1e5, 1e-3))
```

```rust
/* Multiline comment
let ignored = 10;
*/
fn main() {
    let x = 0xff;
    let y = 1_000;
    println!("value: {}", x + y);
}
```

```bash
for file in *.md; do
    echo "$file" # comment
done
```

```yaml
url: https://example.org/a#b
values: [true, null, 1e-8]
notes: |
  # literal text
```

```toml
[markdown]
syntax_highlighting = true
value = 1e-8
text = """first
# still string
"""
```

```json
{"active": true, "value": 1e-8, "name": "sample"}
```

```javascript
const text = `first
// still string`;
function run() { return true; }
```

```typescript
interface Sample { value: number }
const active = true;
```

```sql
SELECT count(*) FROM samples WHERE active = true;
/* comment
continued */
```

```html
<!-- comment
continued -->
<p class="sample">Hello</p>
```

```css
/* comment
continued */
.sample { color: red; opacity: 0.5; }
```

```cpp
int main() { return 0; }
```

```c
int main(void) { return 0; }
```

```typst
#let kd = 1e-8
#text(fill: red)[Sample]
// comment
```

```latex
\frac{k_d}{k_a} % comment
\textbf{Sample}
```

```unknown
let x = 1 # plain fallback
```

# Markdown resumes
**Bold** and *italic*.
