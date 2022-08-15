# gqlmapi-httpserver-proto


Prototype of an HTTP server that wraps a [gqlmapi](https://github.com/microsoft/gqlmapi) service instance, supporting HTTP-based GraphQL query execution per the approach described in [Serving over HTTP](https://graphql.org/learn/serving-over-http). Leverages [gqlmapi-rs](https://github.com/wravery/gqlmapi-rs) as Rust bindings for the underlying C++ gqlmapi library, and [actix-web](https://actix.rs/) as the web framework.

The web server also hosts a GraphiQL endpoint to facilitate easier query crafting and schema exploration.

## Getting Started



## Dependencies

