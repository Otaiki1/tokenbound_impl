# `@crowdpass/tokenbound-sdk`

Typed internal SDK for the CrowdPass Soroban contracts.

## What it includes

- Generated contract metadata derived from the Rust contract interfaces
- Typed wrappers for Event Manager, Ticket Factory, Ticket NFT, TBA Registry, and TBA Account
- Shared transaction builders for read, simulate, sign, and submit flows
- Contract error decoding into SDK-friendly error objects
- **Automatic retry policy with exponential backoff for RPC calls**

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
  // Optional: Configure retry policy for RPC calls
  retryConfig: {
    maxRetries: 3,
    initialDelayMs: 1000,
    maxDelayMs: 30000,
    backoffMultiplier: 2,
    enableJitter: true,
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

### Retry Policy

The SDK automatically retries failed RPC calls with exponential backoff and jitter. This handles transient network failures, rate limiting, and temporary service unavailability.

**Key Features:**
- Automatic retries for transient errors (network issues, timeouts, 5xx errors)
- Exponential backoff with configurable parameters
- Jitter to prevent thundering herd problems
- Smart error detection (only retries appropriate errors)

See [RETRY_POLICY.md](./RETRY_POLICY.md) for detailed documentation.

### Regenerating contract metadata

```bash
cd soroban-client
npm run sdk:generate-types
```
