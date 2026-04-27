import "@testing-library/jest-dom";
import { TextDecoder, TextEncoder } from "node:util";

// Default test values for env vars validated by lib/env.ts. Tests that need
// different values can overwrite these before importing modules that read them.
process.env.NEXT_PUBLIC_HORIZON_URL ??= "https://horizon.example";
process.env.NEXT_PUBLIC_SOROBAN_RPC_URL ??= "https://rpc.example";
process.env.NEXT_PUBLIC_NETWORK_PASSPHRASE ??=
  "Test SDF Network ; September 2015";
process.env.NEXT_PUBLIC_EVENT_MANAGER_CONTRACT ??= "CTEST";

// Some transitive deps (e.g. stellar-sdk) assume these globals exist.
globalThis.TextEncoder ??= TextEncoder;
globalThis.TextDecoder ??= TextDecoder;
