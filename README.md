# Hapi
This is Hapi, the Happy API. Toy project that implements an API gateway.
As such, it's just for fun :-)

## Supported use cases (so far...)
- Upstream lookup: looks for the best available upstream to forward a request to
- Enable upstream: enables the given upstream for all the configured routes
- Disable upstream: disables the given upstream for all the configured routes
- Add route: adds a given route to the current context
- Delete route: deletes a given route to the current context

## Build
```
cargo build --release
```