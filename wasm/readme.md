PeLite WASM bindings
====================

Attempt to bring PeLite to JS via WASM.

Some experimental ideas:

```js
// Implement in wasm
function PeFile(bytes) { .. }
var pe = new PeFile(bytes);
var nt_headers = pe.nt_headers();
```

At the Rust end something like this:

```rust
struct PeFile {
	bytes: Vec<u8>,
	file_name: String,
}
```
