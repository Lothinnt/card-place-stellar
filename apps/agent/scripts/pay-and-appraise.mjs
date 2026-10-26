// Client : l'agent d'un collectionneur paie l'expertise à l'appel, en USDC sur Stellar.
//
//   AGENT_SECRET_KEY=S... node scripts/pay-and-appraise.mjs PSA137798077 [http://localhost:3000]
//
// Flux : POST /api/appraise → 402 + PAYMENT-REQUIRED → le wallet d'agent vérifie
// son plafond, signe une entrée d'autorisation Soroban (transfert USDC) → nouvel
// envoi avec PAYMENT-SIGNATURE → le facilitator règle on-chain → 200 + PAYMENT-RESPONSE.
import "dotenv/config";
import { decodePaymentResponseHeader } from "@x402/fetch";
import { payerFromEnv } from "../src/payer.js";

const [assetCode = "PSA137798077", baseUrl = "http://localhost:3000"] = process.argv.slice(2);

const payer = payerFromEnv();
console.log(`Agent payeur : ${payer.address}`);

const response = await payer.fetch(`${baseUrl}/api/appraise`, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ asset_code: assetCode }),
});

const body = await response.json();
console.log(`HTTP ${response.status}`);
console.log(JSON.stringify(body, null, 2));

const receipt = response.headers.get("PAYMENT-RESPONSE");
if (receipt) {
  const decoded = decodePaymentResponseHeader(receipt);
  console.log("\nReçu de paiement x402 :", JSON.stringify(decoded, null, 2));
  if (decoded.transaction) {
    console.log(`https://stellar.expert/explorer/testnet/tx/${decoded.transaction}`);
  }
}
