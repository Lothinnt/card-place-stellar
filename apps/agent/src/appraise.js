// L'agent IA : relit l'étiquette du gradeur sur la photo, la confronte à la
// fiche on-chain, et donne une estimation indicative. Il ne signe rien et
// ne détient aucune clé : il propose, le détenteur et le dépositaire disposent.
import Anthropic from "@anthropic-ai/sdk";
import { zodOutputFormat } from "@anthropic-ai/sdk/helpers/zod";
import { z } from "zod";
import { config } from "./config.js";

export const Appraisal = z.object({
  label_read: z.object({
    grader: z.string().describe("Gradeur lu sur l'étiquette (ex. PSA)"),
    cert: z.string().describe("Numéro de certificat lu sur l'étiquette"),
    grade: z.string().describe("Note lue sur l'étiquette (ex. GEM MT 10)"),
    title: z.string().describe("Titre complet lu sur l'étiquette"),
  }),
  label_matches_onchain: z
    .boolean()
    .describe("Vrai si gradeur, certificat, note et description on-chain correspondent à l'étiquette"),
  condition_notes: z.string().describe("Observations visuelles : centrage, boîtier, défauts éventuels"),
  estimated_value_eur: z.object({
    low: z.number().describe("Borne basse, en euros"),
    high: z.number().describe("Borne haute, en euros"),
  }),
  confidence: z.enum(["low", "medium", "high"]),
  recommendation: z.enum(["buy", "hold", "sell", "needs_human_review"]),
  rationale: z.string().describe("Justification en 3 phrases maximum"),
});

const SYSTEM = `Tu es l'agent d'expertise de Card Place, une plateforme où des cartes Pokémon gradées
sont conservées en coffre et représentées par un jeton sur Stellar.

Tu reçois : la fiche on-chain d'une carte (gradeur, certificat, note, description) et la photo
de référence dont l'empreinte SHA-256 est inscrite dans le contrat.

Ta mission :
1. Lire l'étiquette du gradeur sur la photo et la retranscrire fidèlement.
2. Dire si l'étiquette correspond à la fiche on-chain. Toute divergence vaut needs_human_review.
3. Décrire l'état visible (centrage, boîtier, rayures, défauts).
4. Donner une fourchette de valeur indicative en euros, prudente, et ton niveau de confiance.

Règles : tu n'affirmes rien qui ne soit visible sur la photo ou fourni dans la fiche. Tu ne
demandes jamais de clé ou de secret. Ton estimation est une information, pas un conseil
d'investissement ; la décision reste au détenteur.`;

/**
 * @param {{ card: object, photo: Buffer, photoVerified: boolean }} input
 * @returns {Promise<z.infer<typeof Appraisal>>}
 */
export async function appraise({ card, photo, photoVerified }) {
  const client = new Anthropic();

  const response = await client.messages.parse({
    model: config.model,
    max_tokens: 4096,
    system: SYSTEM,
    messages: [
      {
        role: "user",
        content: [
          {
            type: "image",
            source: { type: "base64", media_type: "image/jpeg", data: photo.toString("base64") },
          },
          {
            type: "text",
            text: [
              "Fiche on-chain (contrat du coffre Card Place) :",
              JSON.stringify(card, null, 2),
              "",
              `Empreinte de la photo vérifiée contre le contrat : ${photoVerified ? "oui" : "NON"}.`,
              "Analyse la carte et réponds au format demandé.",
            ].join("\n"),
          },
        ],
      },
    ],
    output_config: { format: zodOutputFormat(Appraisal) },
  });

  if (response.stop_reason === "refusal") {
    throw new Error("Le modèle a refusé la demande");
  }
  if (!response.parsed_output) {
    throw new Error("Réponse du modèle non conforme au schéma");
  }
  return response.parsed_output;
}
