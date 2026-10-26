// x402 sur Stellar : le middleware exige un paiement USDC avant chaque appel payant.
// Le client signe une entrée d'autorisation Soroban ; le facilitator vérifie,
// paie les frais et règle on-chain ; la route ne s'exécute qu'après.
import { HTTPFacilitatorClient } from "@x402/core/server";
import { paymentMiddleware, x402ResourceServer } from "@x402/express";
import { ExactStellarScheme } from "@x402/stellar/exact/server";
import { config } from "./config.js";

/**
 * @param {Record<string, { description: string }>} routes  ex. { "POST /api/appraise": {...} }
 * @returns {import("express").RequestHandler}
 */
export function createPaywall(routes) {
  const headers = config.facilitatorApiKey
    ? { Authorization: `Bearer ${config.facilitatorApiKey}` }
    : undefined;

  const facilitator = new HTTPFacilitatorClient({
    url: config.facilitatorUrl,
    createAuthHeaders: headers
      ? async () => ({ verify: headers, settle: headers, supported: headers })
      : undefined,
  });

  const resourceServer = new x402ResourceServer(facilitator).register(
    config.network,
    new ExactStellarScheme(),
  );

  const routesConfig = Object.fromEntries(
    Object.entries(routes).map(([route, { description }]) => [
      route,
      {
        accepts: [
          {
            scheme: "exact",
            price: config.price,
            network: config.network,
            payTo: config.payTo,
          },
        ],
        description,
        mimeType: "application/json",
      },
    ]),
  );

  return paymentMiddleware(routesConfig, resourceServer);
}
