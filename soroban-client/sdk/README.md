# `@crowdpass/tokenbound-sdk`

Typed internal SDK for the CrowdPass Soroban contracts.

## What it includes

- Generated contract metadata derived from the Rust contract interfaces
- Typed wrappers for Event Manager, Ticket Factory, Ticket NFT, TBA Registry, and TBA Account
- Shared transaction builders for read, simulate, sign, and submit flows
- Contract error decoding into SDK-friendly error objects
- **Typed decoder utilities for safe contract response parsing**

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

### Typed Decoders

The SDK provides typed decoder utilities for safely parsing contract responses:

```ts
import { ContractDecoder, decodeArray, decodeStruct, decodeU32, decodeString, decodeI128 } from "./src";

// Decode event response
const event = ContractDecoder.event()(rawResponse);

// Decode array of tiers
const tiers = decodeArray(ContractDecoder.ticketTier())(rawTiers);

// Build custom decoders
const decodeCustom = decodeStruct({
  id: decodeU32,
  name: decodeString,
  price: decodeI128,
});
```

**Key Features:**
- Type-safe contract response parsing
- Composable decoders for complex structures
- Clear error messages with context
- Built-in Soroban type support (u32, u64, u128, i128, etc.)
- Pre-built decoders for contract types

See [DECODERS.md](./DECODERS.md) for detailed documentation.
