# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Carbide Node is a Rust-based backend service that acts as a connecting node for desktop and mobile clients to store and manage large data. It serves as the central storage and data management layer for the Carbide ecosystem.

## Architecture

The project is inspired by [rqbit](https://github.com/ikatson/rqbit), a BitTorrent client written in Rust, particularly for its efficient networking, data handling, and client-server communication patterns.

## Current State

This repository is in its initial setup phase. The codebase currently contains:
- Basic README documentation
- Git repository initialization
- No Rust source code or configuration files yet

## Related Components

- **Mobile Client**: Located at `/Users/chaalpritam/Blockbase/Carbide`
- **Desktop Client**: Located at `/Users/chaalpritam/Blockbase/CarbideDrive`

## Development Setup (When Implemented)

Since this is a Rust project, future development will likely involve:
- `cargo build` - Build the project
- `cargo test` - Run tests
- `cargo run` - Run the application
- `cargo clippy` - Lint checking
- `cargo fmt` - Code formatting

## Key Design Goals

Based on the README, the node should handle:
- Large data storage and retrieval
- Synchronization operations between clients
- Client-server communication for desktop and mobile applications