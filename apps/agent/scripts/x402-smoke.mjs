// Test d'intégration du paywall x402 sur Stellar, sans l'agent IA.
//
// Monte un serveur Express éphémère avec une route payante triviale, puis
// l'appelle avec le wallet d'agent plafonné. Si le facilitator vérifie et règle
// le paiement on-chain, la réponse est 200 et porte un reçu PAYMENT-RESPONSE.
//
//   AGENT_SECRET_KEY=S... node scripts/x402-smoke.mjs
import "dotenv/config";
import express from "express";
import { decodePaymentResponseHeader } from "@x402/fetch";
import { config } from "../src/config.js";
import { createPaywall } from "../src/paywall.js";
import { payerFromEnv } from "../src/payer.js";

// --- serveur éphémère ---
const app = express();
app.use(createPaywall({ "GET /paid/ping": { description: "Smoke test x402" } }));
app.get("/paid/ping", (_req, res) => res.json({ pong: true, at: new Date().toISOString() }));
const server = await new Promise((resolve) => {
  const s = app.listen(0, () => resolve(s));
});
const baseUrl = `http://localhost:${server.address().port}`;
console.log(`Serveur de test sur ${baseUrl} — prix`, JSON.stringify(config.price), "→", config.payTo);

// --- client payeur ---
const payer = payerFromEnv();
console.log(`Payeur : ${payer.address}`);

try {
  const unpaid = await fetch(`${baseUrl}/paid/ping`);
  console.log(`Sans paiement : HTTP ${unpaid.status}`);

  const paid = await payer.fetch(`${baseUrl}/paid/ping`);
  console.log(`Avec paiement : HTTP ${paid.status}`, await paid.json());

  const receipt = paid.headers.get("PAYMENT-RESPONSE");
  if (!receipt) throw new Error("Pas de reçu PAYMENT-RESPONSE");
  const decoded = decodePaymentResponseHeader(receipt);
  console.log("Reçu :", JSON.stringify(decoded, null, 2));
  if (decoded.transaction) {
    console.log(`https://stellar.expert/explorer/testnet/tx/${decoded.transaction}`);
  }
  process.exitCode = paid.status === 200 && decoded.success !== false ? 0 : 1;
} finally {
  server.close();
}
