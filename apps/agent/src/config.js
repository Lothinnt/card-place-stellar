// Configuration centralisée : tout vient de l'environnement, rien n'est codé en dur.
import "dotenv/config";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));

function required(name) {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`Variable d'environnement manquante : ${name}`);
  return value;
}

const network = process.env.STELLAR_NETWORK_CAIP2?.trim() || "stellar:testnet";

const DEFAULTS = {
  "stellar:testnet": {
    rpcUrl: "https://soroban-testnet.stellar.org",
    passphrase: "Test SDF Network ; September 2015",
    facilitatorUrl: "https://channels.openzeppelin.com/x402/testnet",
  },
  "stellar:pubnet": {
    rpcUrl: undefined, // obligatoire en mainnet, à fournir
    passphrase: "Public Global Stellar Network ; September 2015",
    facilitatorUrl: "https://channels.openzeppelin.com/x402",
  },
};

if (!DEFAULTS[network]) {
  throw new Error(`Réseau non supporté : ${network} (attendu stellar:testnet ou stellar:pubnet)`);
}

export const config = Object.freeze({
  port: Number(process.env.PORT ?? 3000),

  // Stellar
  network,
  rpcUrl: process.env.STELLAR_RPC_URL?.trim() || DEFAULTS[network].rpcUrl,
  networkPassphrase: DEFAULTS[network].passphrase,
  contractId: required("VAULT_CONTRACT_ID"),

  // x402 : qui reçoit le paiement, combien, via quel facilitator
  payTo: required("X402_PAY_TO"),
  // Prix : soit un montant en dollars ("$0.01", converti en USDC), soit un
  // actif SEP-41 explicite (X402_ASSET = adresse du contrat, X402_AMOUNT = unités de base, 7 décimales).
  price: process.env.X402_ASSET?.trim()
    ? { asset: process.env.X402_ASSET.trim(), amount: required("X402_AMOUNT") }
    : process.env.X402_PRICE?.trim() || "$0.01",
  facilitatorUrl: process.env.X402_FACILITATOR_URL?.trim() || DEFAULTS[network].facilitatorUrl,
  facilitatorApiKey: process.env.X402_FACILITATOR_API_KEY?.trim() || undefined,

  // Agent IA
  model: process.env.ANTHROPIC_MODEL?.trim() || "claude-opus-5-5",

  // Photos de référence des cartes (recto), nommées psa-<cert>-front.jpg
  photosDir: path.resolve(here, process.env.PHOTOS_DIR?.trim() || "../../../docs"),
});

if (!config.rpcUrl) {
  throw new Error("STELLAR_RPC_URL est obligatoire en mainnet");
}
