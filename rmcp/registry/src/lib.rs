//! API client for the official MCP registry.
//!
//! Types are generated at compile time from the MCP Registry
//! [`server.schema.json`](https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json)
//! using [`typify`](https://github.com/oxidecomputer/typify).
//!
//! To update the types, replace `schemas/server.schema.json` and rebuild.

include!(concat!(env!("OUT_DIR"), "/server_schema.rs"));
