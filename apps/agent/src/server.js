// Serveur HTTP de l'agent Card Place.
//
//   GET  /health                 gratuit  — état du service
//   GET  /vault/:asset_code      gratuit  — fiche on-chain d'une carte (lecture publique)
//   POST /api/appraise           payant   — expertise IA, 0,01 USDC via x402 sur Stellar
//
// Le serveur ne détient aucune clé Stellar : il ne fait que lire le contrat et
// recevoir des paiements sur l'adresse X402_PAY_TO.
import express from "express";
import { appraise } from "./appraise.js";
import { config } from "./config.js";
import { createPaywall } from "./paywall.js";
import { loadReferencePhoto } from "./photos.js";
import { CardNotFoundError, getCard } from "./vault.js";

const app = express();
app.use(express.json({ limit: "16kb" }));

// La page demo-web-page lit les routes gratuites depuis un autre port.
app.use((_req, res, next) => {
  res.set("Access-Control-Allow-Origin", "*");
  next();
});

app.get("/health", (_req, res) => {
  res.json({
    status: "ok",
    service: "card-place-agent",
    network: config.network,
    contract: config.contractId,
    price: config.price,
  });
});

app.get("/vault/:asset_code", async (req, res, next) => {
  try {
    res.json(await getCard(req.params.asset_code));
  } catch (err) {
    next(err);
  }
});

app.use(
  createPaywall({
    "POST /api/appraise": {
      description: "Expertise IA d'une carte gradée conservée dans le coffre Card Place",
    },
  }),
);

app.post("/api/appraise", async (req, res, next) => {
  try {
    const assetCode = String(req.body?.asset_code ?? "").trim();
    if (!/^[A-Za-z0-9]{1,12}$/.test(assetCode)) {
      return res.status(400).json({ error: "asset_code invalide (1 à 12 caractères alphanumériques)" });
    }

    const card = await getCard(assetCode);
    const photo = await loadReferencePhoto(card);
    const appraisal = await appraise({ card, photo: photo.data, photoVerified: photo.verified });

    res.json({
      asset_code: assetCode,
      onchain: card,
      photo_hash_verified: photo.verified,
      appraisal,
      next_step:
        "L'agent propose, il ne signe pas. Le détenteur garde le jeton, le vend sur le carnet d'ordres Stellar, ou appelle redeem() pour récupérer la carte.",
    });
  } catch (err) {
    next(err);
  }
});

// Gestion d'erreurs : une seule place, des codes explicites.
app.use((err, _req, res, _next) => {
  if (err instanceof CardNotFoundError) return res.status(404).json({ error: err.message });
  if (err?.code === "ENOENT") return res.status(404).json({ error: "Photo de référence introuvable" });
  console.error(err);
  res.status(502).json({ error: err.message ?? "Erreur interne" });
});

app.listen(config.port, () => {
  console.log(`Card Place agent sur http://localhost:${config.port} (${config.network}, contrat ${config.contractId})`);
});
