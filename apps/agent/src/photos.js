// Photos de référence : le fichier local dont le SHA-256 est inscrit on-chain.
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { config } from "./config.js";

/**
 * Charge la photo recto de référence d'une carte et vérifie son empreinte
 * contre le hash enregistré dans le contrat.
 * @param {{ grader: string, cert: string, photo_hash: string }} card
 * @returns {Promise<{ data: Buffer, sha256: string, verified: boolean, file: string }>}
 */
export async function loadReferencePhoto(card) {
  const file = path.join(
    config.photosDir,
    `${card.grader.toLowerCase()}-${card.cert}-front.jpg`,
  );
  const data = await readFile(file);
  const sha256 = createHash("sha256").update(data).digest("hex");
  return { data, sha256, verified: sha256 === card.photo_hash, file };
}
