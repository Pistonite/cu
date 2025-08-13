# cu

Copper (Cu = Common utils) is my battery-included utils to quickly
setup CLI applications, like build scripts/test harness sort of stuff.

Doc available at: https://cu.pistonite.dev

Since crates.io does not have namespaces, this crate has a prefix.
You should manually rename it to `cu`, as that's what the proc-macros
expect.
```toml
# Cargo.toml
# ...
[dependencies.cu]
package = "pistonite-cu"
version = "..." # check by running `cargo info pistonite-cu`
features = [ "full" ] # see docs

# ...
[dependencies]
```
