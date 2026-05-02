# gRPC Server Implementation

This crate will implement the gRPC server for the HL7v2 toolkit.

## Overview

Issue #157 identified that a comprehensive protobuf schema exists at `api/proto/hl7v2.proto`
but has no implementation. This branch will implement the gRPC service.

## Implementation Plan

1. Add tonic/prost dependencies
2. Set up build.rs for protobuf code generation
3. Implement HL7Service trait
4. Integrate with existing HTTP server

## Proto Schema

The proto file defines 6 RPCs:
- Parse
- Validate
- GenerateAck
- Normalize
- ParseStream (streaming)
- HealthCheck

## Status

🚧 Work in progress - see PR for details.
