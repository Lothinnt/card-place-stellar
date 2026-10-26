// Le wallet d'agent plafonné : un client x402 qui paie en USDC sur Stellar,
// avec un plafond par paiement imposé côté client avant toute signature.
// L'agent ne peut pas dépenser plus que ce que son opérateur a autorisé.
import { wrapFetchWithPayment, x402Client } from "@x402/fetch";
import { createEd25519Signer } from "@x402/stellar";
import { ExactStellarScheme } from "@x402/stellar/exact/client";

/**
 * @param {object} opts
 * @param {string} opts.secretKey        clé secrète Stellar du compte payeur (S…)
 * @param {string} [opts.network]        CAIP-2, défaut stellar:testnet
 * @param {string} [opts.maxUsdPerPayment]  plafond par paiement pour les actifs par défaut (USDC), ex. "$0.05"
 * @param {{ asset: string, maxAmount?: string }[]} [opts.extraAssets]
 *        actifs SEP-41 supplémentaires autorisés (ex. XLM natif), plafond en unités de base
 * @returns {{ client: x402Client, fetch: typeof fetch, address: string }}
 */
export function createPayer({
  secretKey,
  network = "stellar:testnet",
  maxUsdPerPayment = "$0.05",
  extraAssets = [],
}) {
  if (!secretKey) throw new Error("Clé secrète du payeur manquante");

  const signer = createEd25519Signer(secretKey, network);
  const client = x402Client.fromConfig({
    schemes: [{ network: "stellar:*", client: new ExactStellarScheme(signer) }],
    spendControls: {
      maxAmountPerPayment: maxUsdPerPayment,
      allowedAssets: extraAssets.map(({ asset, maxAmount }) => ({
        network,
        asset,
        maxAmountPerPayment: maxAmount,
      })),
    },
  });

  return { client, fetch: wrapFetchWithPayment(fetch, client), address: signer.address };
}

/** Lit la configuration du payeur depuis l'environnement. */
export function payerFromEnv(env = process.env) {
  const extraAssets = env.AGENT_ALLOWED_ASSET?.trim()
    ? [{ asset: env.AGENT_ALLOWED_ASSET.trim(), maxAmount: env.AGENT_ALLOWED_ASSET_MAX?.trim() }]
    : [];
  return createPayer({
    secretKey: env.AGENT_SECRET_KEY,
    network: env.STELLAR_NETWORK_CAIP2?.trim() || "stellar:testnet",
    maxUsdPerPayment: env.AGENT_MAX_USD_PER_PAYMENT?.trim() || "$0.05",
    extraAssets,
  });
}
