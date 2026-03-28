import { env } from "./env";
import {
  createTokenboundSdk,
  type EventRecord,
  type SignTransactionFn,
  type SorobanSubmitResult,
} from "../sdk/src";

export interface CreateEventParams {
  organizer: string;
  theme: string;
  eventType: string;
  startTimeUnix: number;
  endTimeUnix: number;
  ticketPrice: bigint;
  totalTickets: bigint;
  paymentToken: string;
}

export interface PurchaseTicketParams {
  buyer: string;
  eventId: number;
  tierIndex?: number;
}

export interface PurchaseTicketsParams {
  buyer: string;
  eventId: number;
  quantity: bigint;
}

export interface Event {
  id: number;
  theme: string;
  organizer: string;
  event_type: string;
  total_tickets: bigint;
  tickets_sold: bigint;
  ticket_price: bigint;
  start_date: number;
  end_date: number;
  is_canceled: boolean;
  ticket_nft_addr: string;
  payment_token: string;
}

export interface UpdateEventParams {
  organizer: string;
  event_id: number;
  theme?: string;
  ticket_price?: bigint;
  total_tickets?: bigint;
  start_date?: number;
  end_date?: number;
}

const sdk = createTokenboundSdk({
  horizonUrl: env.NEXT_PUBLIC_HORIZON_URL,
  sorobanRpcUrl: env.NEXT_PUBLIC_SOROBAN_RPC_URL,
  networkPassphrase: env.NEXT_PUBLIC_NETWORK_PASSPHRASE,
  simulationSource: process.env.NEXT_PUBLIC_SOROBAN_SIM_SOURCE,
  contracts: {
    eventManager: env.NEXT_PUBLIC_EVENT_MANAGER_CONTRACT,
    ticketFactory: process.env.NEXT_PUBLIC_TICKET_FACTORY_CONTRACT,
    ticketNft: process.env.NEXT_PUBLIC_TICKET_NFT_CONTRACT,
    tbaRegistry: process.env.NEXT_PUBLIC_TBA_REGISTRY_CONTRACT,
    tbaAccount: process.env.NEXT_PUBLIC_TBA_ACCOUNT_CONTRACT,
  },
});

function normalizeEvent(record: EventRecord): Event {
  return {
    id: record.id,
    theme: record.theme,
    organizer: record.organizer,
    event_type: record.eventType,
    total_tickets: record.totalTickets,
    tickets_sold: record.ticketsSold,
    ticket_price: record.ticketPrice,
    start_date: record.startDate,
    end_date: record.endDate,
    is_canceled: record.isCanceled,
    ticket_nft_addr: record.ticketNftAddress,
    payment_token: record.paymentToken,
  };
}

export function isEventManagerConfigured() {
  return (
    sdk.hasContract("eventManager") &&
    env.NEXT_PUBLIC_EVENT_MANAGER_CONTRACT !== "<MISSING_CONTRACT_ID>"
  );
}

export function getTxExplorerUrl(txHash: string): string {
  return sdk.getExplorerUrl(txHash);
}

export async function createEvent(
  params: CreateEventParams,
  signTransactionFn: SignTransactionFn
): Promise<SorobanSubmitResult> {
  return sdk.eventManager.createEventLegacy(
    {
      organizer: params.organizer,
      theme: params.theme,
      eventType: params.eventType,
      startDate: params.startTimeUnix,
      endDate: params.endTimeUnix,
      ticketPrice: params.ticketPrice,
      totalTickets: params.totalTickets,
      paymentToken: params.paymentToken,
    },
    {
      source: params.organizer,
      signTransaction: signTransactionFn,
    }
  );
}

export async function purchaseTicket(
  params: PurchaseTicketParams,
  signTransactionFn: SignTransactionFn
): Promise<SorobanSubmitResult> {
  return sdk.eventManager.purchaseTicket(params, {
    source: params.buyer,
    signTransaction: signTransactionFn,
  });
}

export async function purchaseTickets(
  params: PurchaseTicketsParams,
  signTransactionFn: SignTransactionFn
): Promise<SorobanSubmitResult> {
  return sdk.eventManager.purchaseTickets(params, {
    source: params.buyer,
    signTransaction: signTransactionFn,
  });
}

export async function getAllEvents(simulationSource?: string | null): Promise<Event[]> {
  if (!isEventManagerConfigured()) {
    return [];
  }
  const events = await sdk.eventManager.getAllEvents({ simulationSource });
  return events.map(normalizeEvent);
}

export async function cancelEvent(
  organizer: string,
  eventId: number,
  signTransactionFn: SignTransactionFn
) {
  return sdk.eventManager.cancelEvent(eventId, {
    source: organizer,
    signTransaction: signTransactionFn,
  });
}

export async function updateEvent(
  params: UpdateEventParams,
  signTransactionFn: SignTransactionFn
) {
  return sdk.eventManager.updateEvent(
    {
      organizer: params.organizer,
      eventId: params.event_id,
      theme: params.theme,
      ticketPrice: params.ticket_price,
      totalTickets: params.total_tickets,
      startDate: params.start_date,
      endDate: params.end_date,
    },
    {
      source: params.organizer,
      signTransaction: signTransactionFn,
    }
  );
}

export async function claimFunds(
  organizer: string,
  eventId: number,
  signTransactionFn: SignTransactionFn
) {
  return sdk.eventManager.withdrawFunds(eventId, {
    source: organizer,
    signTransaction: signTransactionFn,
  });
}

export async function getEventAttendees(
  eventId: number,
  simulationSource?: string | null
): Promise<string[]> {
  if (!isEventManagerConfigured()) {
    return [];
  }
  return sdk.eventManager.getEventBuyers(eventId, { simulationSource });
}
