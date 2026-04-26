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

### Batch ledger-entry fetch

`batchGetLedgerEntries` wraps `rpc.Server.getLedgerEntries` so callers can
read many keys in one workflow. It chunks over the RPC's per-call key
limit, runs chunks concurrently, aligns missing entries to `null` at
their input index, and surfaces per-chunk RPC failures in a structured
`errors` array instead of bubbling the first one and losing the rest.

```ts
import { batchGetLedgerEntries } from "@crowdpass/tokenbound-sdk";

const result = await batchGetLedgerEntries(sdk.rpcServer, ledgerKeys);

for (let i = 0; i < ledgerKeys.length; i += 1) {
  const entry = result.entries[i];
  if (entry === undefined) {
    // chunk failed — see result.errors
  } else if (entry === null) {
    // key not present on-chain
  } else {
    // entry.val is the ledger data
  }
}

// {found, missing, failed, latestLedger} are aggregated for monitoring.
```

`chunkSize`, `concurrency`, and `keyId` are all overridable.
