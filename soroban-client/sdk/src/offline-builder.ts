import {
  Account,
  Operation,
  TransactionBuilder,
  Transaction,
  xdr,
} from "@stellar/stellar-sdk";
import { nativeToScVal } from "@stellar/stellar-base";

import type { ContractCallArtifact, PreparedTransaction } from "./types";

const DEFAULT_FEE = 100;
const DEFAULT_TIMEOUT = 30;

export interface OfflineAccountStub {
  readonly accountId: string;
  readonly sequenceNumber: string;
}

export interface OfflineBuildOptions {
  readonly fee?: number;
  readonly timeoutInSeconds?: number;
}

export interface SerializedTransaction {
  readonly xdr: string;
  readonly networkPassphrase: string;
  readonly source: string;
  readonly sequenceNumber: string;
}

/**
 * Builds an unsigned transaction XDR from a contract call artifact
 * using a pre-fetched account stub — no network call required.
 *
 * Use this for offline or delegated signing flows where you already
 * have the account's current sequence number.
 */
export function buildOfflineTransaction(
  account: OfflineAccountStub,
  artifact: ContractCallArtifact,
  networkPassphrase: string,
  options: OfflineBuildOptions = {},
): PreparedTransaction {
  // Account from stellar-sdk validates and manages sequence correctly
  const sdkAccount = new Account(account.accountId, account.sequenceNumber);

  const operation = Operation.invokeContractFunction({
    contract: artifact.contractId,
    function: artifact.method,
    args: [...artifact.args],
  });

  const tx = new TransactionBuilder(sdkAccount, {
    fee: String(options.fee ?? DEFAULT_FEE),
    networkPassphrase,
  })
    .addOperation(operation)
    .setTimeout(options.timeoutInSeconds ?? DEFAULT_TIMEOUT)
    .build();

  return {
    xdr: tx.toXDR(),
    networkPassphrase,
    source: account.accountId,
  };
}

/**
 * Serializes a PreparedTransaction to a portable JSON-safe object
 * that can be handed off to an offline signer or stored for later signing.
 */
export function serializeTransaction(
  prepared: PreparedTransaction,
  sequenceNumber: string,
): SerializedTransaction {
  return {
    xdr: prepared.xdr,
    networkPassphrase: prepared.networkPassphrase,
    source: prepared.source,
    sequenceNumber,
  };
}

/**
 * Deserializes a SerializedTransaction back to a PreparedTransaction
 * so it can be signed and submitted.
 */
export function deserializeTransaction(
  serialized: SerializedTransaction,
): PreparedTransaction {
  // Validate the XDR is parseable before returning
  TransactionBuilder.fromXDR(serialized.xdr, serialized.networkPassphrase);
  return {
    xdr: serialized.xdr,
    networkPassphrase: serialized.networkPassphrase,
    source: serialized.source,
  };
}

/**
 * Converts a PreparedTransaction XDR back into a Transaction object.
 * Useful for inspecting or re-encoding before signing.
 */
export function decodeTransactionXdr(
  prepared: PreparedTransaction,
): Transaction {
  return TransactionBuilder.fromXDR(
    prepared.xdr,
    prepared.networkPassphrase,
  ) as Transaction;
}

/**
 * Inspects a PreparedTransaction and returns a human-readable summary
 * of the contract call it encodes. Useful for offline review/approval UIs.
 */
export function inspectTransaction(prepared: PreparedTransaction): {
  source: string;
  networkPassphrase: string;
  operations: Array<{ type: string; contractId?: string; method?: string }>;
} {
  const tx = decodeTransactionXdr(prepared);

  const operations = tx.operations.map((op) => {
    if (op.type === "invokeHostFunction") {
      try {
        const hostFn = (op as any).func;
        const invokeContract = hostFn.invokeContract?.();
        if (invokeContract) {
          const method = invokeContract.functionName().toString();
          return { type: op.type, method };
        }
      } catch {
        // fall through to generic
      }
    }
    return { type: op.type };
  });

  return {
    source: prepared.source,
    networkPassphrase: prepared.networkPassphrase,
    operations,
  };
}

/**
 * Builds a contract call artifact for offline use without any SDK instance.
 */
export function buildContractArtifact(
  contractId: string,
  method: string,
  args: ReturnType<typeof nativeToScVal>[],
): ContractCallArtifact {
  return { contractId, method, args };
}

/**
 * OfflineTransactionBuilder — a stateless helper class for constructing
 * unsigned transactions without any network dependency.
 *
 * Intended for:
 * - Hardware wallet flows (build -> export XDR -> sign offline -> broadcast)
 * - Delegated signing (build on server -> sign on client)
 * - Air-gapped environments
 */
export class OfflineTransactionBuilder {
  private readonly networkPassphrase: string;

  constructor(networkPassphrase: string) {
    this.networkPassphrase = networkPassphrase;
  }

  /** Builds an unsigned PreparedTransaction. Makes zero network calls. */
  build(
    account: OfflineAccountStub,
    artifact: ContractCallArtifact,
    options: OfflineBuildOptions = {},
  ): PreparedTransaction {
    return buildOfflineTransaction(
      account,
      artifact,
      this.networkPassphrase,
      options,
    );
  }

  /** Builds and immediately serializes for transport/storage. */
  buildAndSerialize(
    account: OfflineAccountStub,
    artifact: ContractCallArtifact,
    options: OfflineBuildOptions = {},
  ): SerializedTransaction {
    const prepared = this.build(account, artifact, options);
    return serializeTransaction(prepared, account.sequenceNumber);
  }

  /** Restores a SerializedTransaction back to a PreparedTransaction. */
  restore(serialized: SerializedTransaction): PreparedTransaction {
    return deserializeTransaction(serialized);
  }

  /** Inspects a prepared transaction for human-readable review. */
  inspect(prepared: PreparedTransaction) {
    return inspectTransaction(prepared);
  }
}
