// Lecture du contrat du coffre Card Place : une simulation RPC, aucune signature, aucun frais.
import {
  Account,
  BASE_FEE,
  Contract,
  TransactionBuilder,
  nativeToScVal,
  rpc,
  scValToNative,
} from "@stellar/stellar-sdk";
import { config } from "./config.js";

const STATUS_LABELS = ["InVault", "RedeemRequested", "Shipped"];

export class CardNotFoundError extends Error {
  constructor(assetCode) {
    super(`Aucune carte enregistrée sous le code ${assetCode}`);
    this.name = "CardNotFoundError";
  }
}

/**
 * Lit la fiche d'une carte via `get_card(asset_code)`.
 * @param {string} assetCode ex. "PSA137798077"
 * @returns {Promise<Card>}
 */
export async function getCard(assetCode) {
  const server = new rpc.Server(config.rpcUrl);
  const contract = new Contract(config.contractId);

  // Une simulation a besoin d'un compte source ; on utilise le dépositaire,
  // qui existe forcément. Le numéro de séquence n'a pas d'importance ici.
  const source = new Account(config.payTo, "0");
  const tx = new TransactionBuilder(source, {
    fee: BASE_FEE,
    networkPassphrase: config.networkPassphrase,
  })
    .addOperation(contract.call("get_card", nativeToScVal(assetCode, { type: "symbol" })))
    .setTimeout(30)
    .build();

  const sim = await server.simulateTransaction(tx);
  if (rpc.Api.isSimulationError(sim)) {
    // Error(Contract, #2) = CardNotFound dans le contrat.
    if (/Error\(Contract, #2\)/.test(sim.error)) throw new CardNotFoundError(assetCode);
    throw new Error(`Simulation échouée : ${sim.error}`);
  }

  return normalize(assetCode, scValToNative(sim.result.retval));
}

/** Convertit la structure Soroban en objet JSON lisible. */
function normalize(assetCode, raw) {
  return {
    asset_code: assetCode,
    token: raw.token,
    grader: raw.grader,
    cert: raw.cert,
    grade: Number(raw.grade),
    description: raw.description,
    photo_hash: Buffer.from(raw.photo_hash).toString("hex"),
    status: STATUS_LABELS[Number(raw.status)] ?? String(raw.status),
    redeemer: raw.redeemer ?? null,
    tracking: raw.tracking ?? null,
    registered_at: Number(raw.registered_at),
  };
}

/**
 * @typedef {object} Card
 * @property {string} asset_code
 * @property {string} token
 * @property {string} grader
 * @property {string} cert
 * @property {number} grade
 * @property {string} description
 * @property {string} photo_hash  hex SHA-256
 * @property {"InVault"|"RedeemRequested"|"Shipped"} status
 * @property {string|null} redeemer
 * @property {string|null} tracking
 * @property {number} registered_at
 */
