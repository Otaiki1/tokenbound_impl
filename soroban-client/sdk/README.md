# `@crowdpass/tokenbound-sdk`

Typed internal SDK for the CrowdPass Soroban contracts.

## What it includes

- Generated contract metadata derived from the Rust contract interfaces
- Typed wrappers for Event Manager, Ticket Factory, Ticket NFT, TBA Registry, and TBA Account
- Shared transaction builders for read, simulate, sign, and submit flows
- Contract error decoding into SDK-friendly error objects

## Usage

```ts
import { createTokenboundSdk } from "./src";

const sdk = createTokenboundSdk({
  horizonUrl: process.env.NEXT_PUBLIC_HORIZON_URL!,
  sorobanRpcUrl: process.env.NEXT_PUBLIC_SOROBAN_RPC_URL!,
  networkPassphrase: process.env.NEXT_PUBLIC_NETWORK_PASSPHRASE!,
  simulationSource: process.env.NEXT_PUBLIC_SOROBAN_SIM_SOURCE,
  contracts: {
    eventManager: process.env.NEXT_PUBLIC_EVENT_MANAGER_CONTRACT,
  },
});

const events = await sdk.eventManager.getAllEvents();
```

### Creating an event

```ts
const result = await sdk.eventManager.createEvent(
  {
    organizer: walletAddress,
    theme: "Stellar Builders Summit",
    eventType: "Conference",
    startDate: 1790899200,
    endDate: 1790985600,
    ticketPrice: 1000_0000000n,
    totalTickets: 250n,
    paymentToken: tokenAddress,
  },
  {
    source: walletAddress,
    signTransaction,
  }
);
```

### Regenerating contract metadata

```bash
cd soroban-client
npm run sdk:generate-types
```

### Caching contract schemas at runtime

Soroban contracts in this repo follow the upgradeable pattern, so each
contract's deployed `version()` and WASM hash uniquely identify its
on-chain spec. `ContractSchemaCache` caches a fetched spec keyed by that
identity and refreshes automatically when either component changes:

```ts
import {
  ContractSchemaCache,
  WebStorageSchemaStore,
  type SchemaIdentityProbe,
  type SchemaResolver,
} from "@crowdpass/tokenbound-sdk";

const probe: SchemaIdentityProbe = {
  async probe(contractId) {
    const version = Number(await sdk.eventManager.version());
    const wasmHash = await fetchWasmHash(contractId); // app-specific
    return { version, wasmHash };
  },
};

const resolver: SchemaResolver = {
  async resolve(contractId) {
    return loadSpecFromRpc(contractId); // app-specific
  },
};

const schemaCache = new ContractSchemaCache({
  store: new WebStorageSchemaStore(window.localStorage),
  resolver,
  probe,
});

const spec = await schemaCache.get(contractId);
```

`MemorySchemaStore` is used by default and is enough for server contexts;
`WebStorageSchemaStore` persists across reloads in the browser. Omit
`probe` if you only want to refresh on explicit `cache.refresh(...)` or
`cache.invalidate(...)` calls. Subscribe via `cache.subscribe(listener)`
to observe `hit` / `miss` / `refreshed` / `invalidated` events.
