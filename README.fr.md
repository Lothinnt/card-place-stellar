# Card Place — Acheter et vendre des cartes en toute sécurité

<p align="center"><a href="README.md">🇬🇧 English</a> &nbsp;|&nbsp; <b>🇫🇷 Français</b></p>

**Une bourse sécurisée pour investir dans les cartes à collectionner comme on investit dans une action** : l'argent de l'acheteur est bloqué sous escrow par un contrat Soroban jusqu'à la vérification de la carte, la carte elle-même reste en coffre et s'échange sous forme de jeton Stellar, et chaque vente laisse un historique de prix public et vérifiable.

Construit pour HackMeridian 2026 (Stellar, Lisbonne). Tout ce qui est décrit comme *déployé* ci-dessous tourne sur le testnet Stellar, avec de vraies transactions à ouvrir dans l'explorateur.

## Le problème

Les cartes à collectionner sont devenues un vrai investissement, et le marché grandit vite : 2,8 Md$ en Amérique du Nord et 1,3 Md$ en Europe en 2025 pour les seuls jeux de cartes (Global Market Insights). Un Dracaufeu 1re Édition PSA 10 s'est vendu 1,38 M$ en octobre 2026.

Je suis collectionneur, et cofondateur de [Graded Card Shop](https://gradedcardshop.fr) : nous achetons et vendons des cartes gradées, plus de 300 ventes sans aucune publicité, uniquement sur Vinted et eBay. Au quotidien, ce marché a trois problèmes :

1. **Contrefaçons et arnaques.** Fausses cartes, faux slabs aux étiquettes copiées, boîtes de boosters rescellées et vendues comme neuves. Chaque transaction, c'est faire confiance à un inconnu.
2. **Vols.** Colis volés pendant la livraison, assurances qui ne paient pas toujours, cambriolages qui visent les collectionneurs.
3. **Données éparpillées.** Les ventes ont lieu sur des plateformes qui ne partagent pas leur historique. Pas d'historique de prix fiable, aucun moyen de comparer une carte aux autres ou au marché, comme on le ferait pour une action ou une crypto.

**Card Place vise toutes les cartes, mais commence par les cartes gradées** (PSA, PCA, BGS) : déjà authentifiées, scellées dans un slab, identifiées par un numéro de certificat unique. Leur état est figé et leur prix est liquide. C'est le marché le plus simple à sécuriser en premier.

## Ce que fait Card Place

<p align="center">
  <img src="docs/screen-card.jpg" alt="Card Place — fiche de la carte Zekrom ex PSA 10 : historique des ventes, rareté, ventes récentes, fiche on-chain" width="100%">
</p>

Card Place fait deux choses.

**Elle sécurise l'échange.** Les USDC de l'acheteur sont bloqués dans un **contrat d'escrow**, ni chez le vendeur ni chez Card Place. Le vendeur n'expédie qu'une fois le dépôt visible on-chain. La carte arrive dans notre coffre, où nous l'inspectons. Si c'est bien ce qui a été vendu, le contrat paie le vendeur ; sinon, le contrat rembourse l'acheteur. Si rien ne se passe avant l'échéance, l'acheteur peut se rembourser lui-même. Aucune confiance nécessaire entre acheteur et vendeur, et personne ne peut retenir l'argent.

**Elle donne une vue complète avant d'acheter.** Chaque carte a sa page : historique de prix, ventes récentes, rareté, et sa place par rapport au reste du marché. Les données viennent de la chaîne, donc chacun peut les vérifier.

Le vendeur peut aussi laisser sa carte en coffre. Elle est alors représentée par **un actif Stellar natif** (code = numéro de certificat, 1 unité émise). Le jeton s'échange sur le carnet d'ordres natif en 5 secondes pour une fraction de centime, et la carte physique ne voyage qu'une fois : quand le dernier détenteur appelle `redeem()`, rend le jeton et reçoit la carte.

### Le parcours utilisateur

Acheteur :

1. Je veux cette carte : j'ouvre un escrow et je dépose le prix en USDC. → `escrow.open`
2. Le vendeur expédie la carte à Card Place. Nous l'inspectons, et un agent IA recoupe le slab avec le registre du gradeur.
3. Carte conforme : le contrat paie le vendeur. → `escrow.release`. Non conforme : je récupère mon argent. → `escrow.refund`
4. Je reçois la carte, ou je la laisse en coffre sous forme de jeton que je peux revendre sans l'expédier.

Vendeur :

1. Je mets ma carte en vente à mon prix, et j'accepte ou refuse les offres.
2. Je n'expédie qu'une fois l'argent de l'acheteur bloqué on-chain.
3. Si ma carte est en coffre, c'est un jeton : je peux la vendre autant de fois que je veux sans y toucher.

### La sécurité, concrètement

- **Un escrow sans pouvoir discrétionnaire.** Libération et remboursement suivent les règles écrites dans le contrat : confirmation du dépositaire, ou une échéance qui permet à l'acheteur de se rembourser seul. L'argent est à l'adresse d'un contrat, visible de tous.
- **Aucune IA dans la validation des transactions.** Valider un paiement, libérer l'escrow, brûler un jeton : du code déterministe, testé, lisible. L'IA est ailleurs : un agent de code pour construire et déployer les contrats, et un agent d'expertise payé à l'appel qui ne touche jamais aux fonds.
- **Seul le dépositaire peut créer le jeton d'une carte**, et seulement après l'inspection physique. Une carte qui n'est pas en coffre n'a pas de jeton. Une contrefaçon, une photo volée ou un slab rescellé ne produisent rien.
- **La photo de référence est prise par le dépositaire**, pas par le vendeur, et son SHA-256 est stocké on-chain : personne ne peut la remplacer ensuite.
- **L'agent IA identifie la carte ; il ne remplace pas l'authentification physique.** Il recoupe le registre public du gradeur, l'étiquette du slab lue sur la photo, et la fiche on-chain. Si l'une diverge, un humain regarde.

## Ce qui tourne aujourd'hui sur le testnet

Deux parcours complets on-chain, exécutés avec une vraie carte de notre stock (Zekrom ex #166 BLK FR 2025, PSA 10, certificat 137798077) :

**Le coffre.**
1. Le dépositaire authentifie la carte, la met en coffre et l'enregistre on-chain : certificat, note, SHA-256 de la photo. → `register_card`
2. Le jeton circule librement : paiements, carnet d'ordres, trustline = consentement du receveur.
3. Un agent IA, payé 0,01 USDC par appel via **x402 sur Stellar**, vérifie la photo contre le contrat, lit l'étiquette du slab, la compare à la fiche on-chain et au certificat du gradeur, et note l'état.
4. Le détenteur rend le jeton et demande la carte. → `redeem` (le SAC brûle le jeton)
5. Le dépositaire confirme l'expédition avec le numéro de suivi. → `mark_shipped`

**L'escrow.**
1. L'acheteur dépose 943 USDC pour la carte. → `open` — les USDC quittent le compte de l'acheteur pour celui du contrat.
2. Le dépositaire reçoit et vérifie la carte, et libère les fonds. → `release` — le vendeur reçoit 943 USDC.
3. Un second échange, 50 USDC, pour une carte qui échoue à l'inspection : le dépositaire rembourse. → `refund` — l'acheteur récupère ses 50 USDC.

Les deux contrats sont courts, entièrement commentés, et couverts par des tests unitaires (11 pour le coffre, 13 pour l'escrow).

## Pourquoi Stellar

Remplacez Stellar par une base de données et il faut un intermédiaire de confiance pour tenir les soldes et publier les prix, plus des délais de règlement et des frais à chaque revente. Sur Stellar :

- **L'escrow est un contrat, pas une société.** Soroban garde les USDC ; les règles sont du code public.
- **Un jeton de carte n'est pas un contrat.** Une carte = code d'actif + émetteur, 1 unité. L'émission prend deux transactions.
- **Le marché est déjà dans le protocole.** Le carnet d'ordres natif échange la carte contre XLM ou USDC sans code de marketplace, et il est public : la fin des prix opaques.
- **Chaque vente est une transaction publique.** L'historique de prix d'une carte est vérifiable par n'importe qui, sans dépendre de la plateforme.
- **Une trustline est un consentement.** Personne ne reçoit une carte sans l'avoir acceptée.
- **Les frais rendent viable une carte à 10 €**, ce qui est économiquement impossible ailleurs.
- **Le fiat entre et sort par les anchors** (SEP-24, MoneyGram) : l'acheteur paie en euros, le registre règle en USDC.
- **x402 sur Stellar** permet aux agents de payer dans la même monnaie que les collectionneurs, avec des frais pris en charge par le facilitateur.

[StockX](https://stockx.com) a prouvé le modèle carnet d'ordres + authentification pour les sneakers, sans blockchain. [Courtyard](https://courtyard.io) tokenise des cartes PSA sur Polygon. L'angle de Card Place : la niche européenne (PCA, PSA FR), un vendeur qui est aussi dépositaire, les cartes à petit prix que les frais excluent ailleurs, un escrow qui protège les deux parties, et le prix public comme produit.

## Architecture

<p align="center">
  <img src="docs/architecture.svg" alt="Architecture de Card Place : dépositaire, collectionneur, contrat coffre Soroban, jeton de carte, carnet d'ordres natif, USDC, agent IA d'expertise, facilitateur x402" width="100%">
</p>

Source : [`docs/architecture.excalidraw`](docs/architecture.excalidraw), modifiable sur [excalidraw.com](https://excalidraw.com). Le dépositaire écrit la fiche, le marché vit sur le carnet d'ordres natif, l'escrow garde les USDC entre acheteur et vendeur, l'agent lit la fiche et se fait payer en USDC, et `redeem` brûle le jeton et déclenche l'expédition.

## Specs techniques

### Contrat `contracts/escrow` (Rust, `soroban-sdk` 28, cible `wasm32v1-none`)

| Fonction | Auth | Entrées | Sortie | Effet |
|---|---|---|---|---|
| `__constructor(custodian, token)` | déploiement | `Address, Address` | — | fixe le dépositaire et l'actif de paiement (SAC USDC) |
| `open(buyer, seller, card, amount, deadline_ledgers)` | acheteur | `Address, Address, Symbol, i128, u32` | `Result<u32, Error>` | transfère `amount` de l'acheteur vers le contrat, crée l'échange, état `Funded`. Renvoie l'identifiant. Erreurs `InvalidAmount` (3), `SameParty` (5) |
| `release(trade_id)` | dépositaire | `u32` | `Result<(), Error>` | transfère `amount` du contrat vers le vendeur, état `Released`. Erreurs `TradeNotFound` (1), `InvalidStatus` (2) |
| `refund(caller, trade_id)` | appelant | `Address, u32` | `Result<(), Error>` | rend `amount` à l'acheteur, état `Refunded`. Le dépositaire peut l'appeler à tout moment ; l'acheteur seulement après `deadline`. Erreur `NotExpired` (4) |
| `get_trade(trade_id)` | aucune | `u32` | `Result<Trade, Error>` | lecture publique |

**Fiche `Trade`.** `buyer`, `seller`, `card` (code d'actif de la carte, lien avec le coffre), `amount` (7 décimales), `status`, `opened_at`, `deadline` (ledger).

**Invariants.** Un échange quitte `Funded` exactement une fois, vers `Released` ou `Refunded`. Le montant qui sort du contrat est celui qui a été déposé. Le vendeur ne peut jamais déclencher un remboursement. 13 tests unitaires couvrent les deux chemins, l'échéance et chaque refus.

### Contrat `contracts/vault` (Rust, `soroban-sdk` 28, cible `wasm32v1-none`)

| Fonction | Auth | Entrées | Sortie | Effet |
|---|---|---|---|---|
| `__constructor(custodian)` | déploiement | `Address` | — | fixe le dépositaire |
| `register_card(asset_code, token, grader, cert, grade, description, photo_hash)` | dépositaire | `Symbol, Address, Symbol, Symbol, u32, String, BytesN<32>` | `Result<(), Error>` | crée la fiche, état `InVault`. Erreur `CardAlreadyRegistered` (1) |
| `redeem(holder, asset_code)` | détenteur | `Address, Symbol` | `Result<(), Error>` | transfère 1 unité du jeton (SAC) au dépositaire, état `RedeemRequested`. Erreurs `CardNotFound` (2), `InvalidStatus` (3) |
| `mark_shipped(asset_code, tracking)` | dépositaire | `Symbol, String` | `Result<(), Error>` | état `Shipped`, stocke le suivi |
| `get_card(asset_code)` | aucune | `Symbol` | `Result<Card, Error>` | lecture publique |

**Invariants.** Transitions strictes `InVault → RedeemRequested → Shipped`. `redeem` exige l'auth du détenteur, qui couvre le `transfer` du SAC : personne ne peut rendre le jeton d'un autre. Dépositaire = émetteur, donc le transfert brûle le jeton. 11 tests unitaires.

Les deux contrats gardent leurs fiches en stockage persistant, TTL prolongé à chaque écriture (seuil 30 jours, cible 1 an). Pas de `Vec`, pas de boucle : coût constant par appel.

### Agent `apps/agent` (Node 22, ESM, Express 5)

| Route | Prix | Rôle |
|---|---|---|
| `GET /health` | gratuit | état du service |
| `GET /vault/:asset_code` | gratuit | fiche on-chain, lue par simulation RPC |
| `POST /api/appraise` `{ asset_code }` | 0,01 USDC via x402 | lit la fiche, vérifie l'empreinte de la photo ; Claude lit l'étiquette, la compare à la fiche et au certificat du gradeur, note l'état et estime la valeur |

**x402 sur Stellar.** `@x402/express` + `@x402/stellar`, facilitateur testnet hébergé par OpenZeppelin, USDC SEP-41. Le client signe une entrée d'autorisation Soroban ; le facilitateur reconstruit la transaction, prend en charge les frais et règle. Le client refuse de signer au-delà du plafond `AGENT_MAX_USD_PER_PAYMENT`.

**L'agent IA ne détient aucune clé Stellar et ne signe aucune transaction.** Il propose ; le détenteur et le dépositaire disposent.

## Déployé sur le testnet

| Quoi | Adresse / transaction |
|---|---|
| Dépositaire et déployeur | [`GBNZP4YND7GXOM26YNBOAXIMTEMKNHDX7CQ5VZOJ3VKW7HDJSQR3XK75`](https://stellar.expert/explorer/testnet/account/GBNZP4YND7GXOM26YNBOAXIMTEMKNHDX7CQ5VZOJ3VKW7HDJSQR3XK75) |
| **Contrat escrow** | [`CDZVLUGTZW4CW46XYGIL4BGOFGMQW6HJTHSDMEDMGDEOKN2EMW22IDTM`](https://stellar.expert/explorer/testnet/contract/CDZVLUGTZW4CW46XYGIL4BGOFGMQW6HJTHSDMEDMGDEOKN2EMW22IDTM) |
| Déploiement de l'escrow | [`f60a82…6c336`](https://stellar.expert/explorer/testnet/tx/f60a823a5d7392029b836a0007c6cd24b9be777b5e234d95ba37e4d51636c336) |
| USDC (testnet, SAC) | [`CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA`](https://stellar.expert/explorer/testnet/contract/CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA) |
| Acheteur | [`GCR6HZKROPDQY3KZBSC27KKYEB7RAJRDWEIEUQYIXHUA6D6WOVXTWVHH`](https://stellar.expert/explorer/testnet/account/GCR6HZKROPDQY3KZBSC27KKYEB7RAJRDWEIEUQYIXHUA6D6WOVXTWVHH) |
| Vendeur | [`GDSMQRA65ZTZBFP57USVK6QNFZNZTCAPCTEDUYUKXEMSOIZAGKI4ZUPM`](https://stellar.expert/explorer/testnet/account/GDSMQRA65ZTZBFP57USVK6QNFZNZTCAPCTEDUYUKXEMSOIZAGKI4ZUPM) |
| Échange 1 · `open`, 943 USDC bloqués | [`406af4…c85430`](https://stellar.expert/explorer/testnet/tx/406af426218d25a603bc1319151ef3596eaa1efbd0a084344af45140a3c85430) |
| Échange 1 · `release`, vendeur payé | [`07db8e…b8da7`](https://stellar.expert/explorer/testnet/tx/07db8e46d48ee92d59eb04462bea682453b416c193524542b0da4878180b8da7) |
| Échange 2 · `open`, 50 USDC bloqués | [`a3aa86…805316`](https://stellar.expert/explorer/testnet/tx/a3aa8640135fa935cfb84b96143cbfa355d25a3769255c4796a65c08fe805316) |
| Échange 2 · `refund`, acheteur remboursé | [`1160c8…4f8c00`](https://stellar.expert/explorer/testnet/tx/1160c8477a586b6259553ef9258e673d0bbc27374ade4f0381e8a814e04f8c00) |
| **Contrat coffre** | [`CDN5OOWTOYKEXMH5CDG7APQRQEHGFRKWQEAHQR5XR3643YOM2YEMEMRO`](https://stellar.expert/explorer/testnet/contract/CDN5OOWTOYKEXMH5CDG7APQRQEHGFRKWQEAHQR5XR3643YOM2YEMEMRO) |
| Déploiement du coffre | [`5833d9…978d1`](https://stellar.expert/explorer/testnet/tx/5833d98dd4230adee7f78feb8b34dcf8d8a190cb453c7b0dbf224c1ab25978d1) |
| Actif de la carte `PSA137798077` (SAC) | [`CASKFD5AFFRLUEW6YJUQRSTXJ7MQU7ZMUCMW254G4CR5QJ45Q5DYNVKH`](https://stellar.expert/explorer/testnet/contract/CASKFD5AFFRLUEW6YJUQRSTXJ7MQU7ZMUCMW254G4CR5QJ45Q5DYNVKH) |
| `register_card` (dépositaire) | [`246cf5…6d32c`](https://stellar.expert/explorer/testnet/tx/246cf5ebeebd77fe2176ef1fa584334a9c6f0e0387e8eed37d45a8ed7526d32c) |
| `redeem` (collectionneur, événement `burn` du SAC) | [`fb6a62…8977a`](https://stellar.expert/explorer/testnet/tx/fb6a6235c2cac014ed0e1e846945e1cf5b574b25458ba55e0e9946e0e398977a) |
| `mark_shipped` (dépositaire) | [`342db4…5ed3cf`](https://stellar.expert/explorer/testnet/tx/342db414c65cf3bf04648dde4fd87a280d883b2644c287fd64b733ada25ed3cf) |
| Paiement x402 réglé par le facilitateur | [`4709b9…763c4`](https://stellar.expert/explorer/testnet/tx/4709b937d086eef100fe09bc760bde81f9d84887a7fff12f14beec9268e763c4) |

Le SHA-256 de `docs/psa-137798077-front.jpg` est `e274a654…0abfa`, stocké on-chain.

## Reproduire

Prérequis : Rust + la cible `wasm32v1-none`, [Stellar CLI](https://developers.stellar.org/docs/tools/cli) 28, Node 22, des identités testnet financées (`stellar keys generate moi --network testnet --fund`, idem pour `collectionneur` et `vendeur`).

### 1. Contrats

```sh
cargo test                                   # 24 tests : 11 coffre + 13 escrow
stellar contract build                       # target/wasm32v1-none/release/{card_vault,card_escrow}.wasm

MOI=$(stellar keys address moi)
stellar contract deploy \
  --wasm target/wasm32v1-none/release/card_vault.wasm \
  --source-account moi --network testnet --alias vault \
  -- --custodian "$MOI"

USDC=$(stellar contract id asset --asset USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5 --network testnet)
stellar contract deploy \
  --wasm target/wasm32v1-none/release/card_escrow.wasm \
  --source-account moi --network testnet --alias escrow \
  -- --custodian "$MOI" --token "$USDC"
```

### 2. Le cycle de l'escrow

L'acheteur et le vendeur ont besoin d'une trustline USDC (`stellar tx new change-trust --source-account <qui> --line USDC:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5 --network testnet`), et l'acheteur de quelques USDC testnet ([faucet Circle](https://faucet.circle.com), réseau Stellar).

```sh
COL=$(stellar keys address collectionneur); VEN=$(stellar keys address vendeur)

# l'acheteur bloque 943 USDC pour la carte (échéance ≈ 7 jours de ledgers)
stellar contract invoke --id escrow --source-account collectionneur --network testnet --send=yes -- \
  open --buyer "$COL" --seller "$VEN" --card PSA137798077 --amount 9430000000 --deadline_ledgers 120960
# → 1  (identifiant de l'échange)

# le dépositaire vérifie la carte et paie le vendeur
stellar contract invoke --id escrow --source-account moi --network testnet --send=yes -- release --trade_id 1

# ou, si la carte échoue à l'inspection, rembourse l'acheteur
stellar contract invoke --id escrow --source-account moi --network testnet --send=yes -- refund --caller "$MOI" --trade_id 1

# lecture publique
stellar contract invoke --id escrow --source-account moi --network testnet -- get_trade --trade_id 1
```

### 3. La carte : un actif natif, un SAC, un détenteur

```sh
stellar tx new change-trust --source-account collectionneur --line "PSA137798077:$MOI" --network testnet
stellar tx new payment --source-account moi --destination "$COL" \
  --asset "PSA137798077:$MOI" --amount 10000000 --network testnet          # 1 unité = 1 carte
stellar contract asset deploy --asset "PSA137798077:$MOI" --source-account moi --network testnet --alias card_psa137798077
```

### 4. Le cycle du coffre

```sh
# le dépositaire enregistre la carte (hash = sha256 de docs/psa-137798077-front.jpg)
stellar contract invoke --id vault --source-account moi --network testnet --send=yes -- \
  register_card --asset_code PSA137798077 --token CASKFD5AFFRLUEW6YJUQRSTXJ7MQU7ZMUCMW254G4CR5QJ45Q5DYNVKH \
  --grader PSA --cert '"137798077"' --grade 10 \
  --description '"2025 Pokemon BLK FR Zekrom ex #166 Special Illustration Rare"' \
  --photo_hash e274a6542ed20a6121176b169f09bf56c8a58ea62653109a9d553aa073a0abfa

# le collectionneur rend le jeton et demande l'expédition
stellar contract invoke --id vault --source-account collectionneur --network testnet --send=yes -- \
  redeem --holder "$COL" --asset_code PSA137798077

# le dépositaire confirme l'expédition
stellar contract invoke --id vault --source-account moi --network testnet --send=yes -- \
  mark_shipped --asset_code PSA137798077 --tracking '"LA123456789FR"'
```

Sans `--send=yes`, l'appel est seulement simulé et n'atteint jamais le registre.

### 5. L'agent IA et l'API x402

```sh
cd apps/agent
npm install
cp .env.example .env     # VAULT_CONTRACT_ID, X402_PAY_TO, X402_FACILITATOR_API_KEY, ANTHROPIC_API_KEY, AGENT_SECRET_KEY
```

- Clé gratuite du facilitateur testnet : `curl https://channels.openzeppelin.com/testnet/gen`
- Le compte payeur a besoin d'une trustline USDC et de quelques USDC testnet.

```sh
node scripts/x402-smoke.mjs                       # vrai paiement x402, sans IA : 402 → 200 + reçu
npm start                                         # serveur sur :3000
curl localhost:3000/vault/PSA137798077            # gratuit
node scripts/pay-and-appraise.mjs PSA137798077    # l'agent paie 0,01 USDC et reçoit l'expertise
```

### 6. La marketplace : page web et app mobile

HTML pur, sans framework ni build.

```sh
node demo-web-page/serve.mjs                           # http://localhost:8080/demo-web-page/  et  /mobile-app/
sh demo-web-page/seed-market.sh                        # place un ordre de vente et un ordre d'achat sur le DEX Stellar
```

- **Web** (`demo-web-page/`) : classement par capitalisation par TCG et par carte, fiche Zekrom avec courbe d'historique, rareté, ventes récentes, fiche lue dans le contrat, prix d'achat en direct depuis le carnet d'ordres Stellar.
- **Mobile** (`mobile-app/`) : accueil avec valeur du coffre, exploration par carte ou par TCG, fiche avec historique de prix, coffre, ouverture de packs. À ouvrir en 390 px de large, ou sur un téléphone du même réseau.
- La fiche est lue en direct via l'agent (`GET /vault/:code`) ; le prix d'achat et les transactions du jeton sont lus en direct sur Horizon. Historique des ventes, population, tableau de marché et classement sont des données de démo dans `demo-web-page/data/`.

## Ce qui est réel, ce qui reste

| Dans ce dépôt, vérifié sur le testnet | Ensuite |
|---|---|
| **Contrat d'escrow** : `open` bloque les USDC de l'acheteur, `release` paie le vendeur, `refund` rembourse en cas de litige ou après l'échéance. Les deux chemins exécutés on-chain | Le programme de surveillance, sans IA : écoute le transporteur et les événements on-chain, déclenche `release` ou `refund` selon les règles du contrat ; les litiges vont à un humain |
| Contrat coffre, cycle complet enregistrée → rendue → expédiée avec une vraie carte | Plusieurs vraies cartes de notre stock, échange en direct sur le carnet d'ordres |
| Carte = actif natif + SAC, jeton brûlé au `redeem` | Wallet connecté (Freighter), paiement en fiat via un anchor |
| Paiement x402 sur Stellar réglé on-chain par le facilitateur | L'agent qui achète pour un collectionneur dans sa limite de dépense |
| Agent IA d'expertise : lecture on-chain, contrôle SHA-256, sortie structurée | Vérification du certificat chez le gradeur (API PSA, page PCA) |
| Page web et app mobile : fiche on-chain et carnet d'ordres lus en direct, courbe, population, classement | Acheter et vendre depuis la page, via l'escrow ; historique des ventes indexé depuis les transactions du jeton |
| Ouverture de packs sur mobile (prototype, tirage local) | Tirage vérifiable on-chain |

Ensuite : une empreinte physique de la carte (scan haute résolution sous éclairage fixe, hash on-chain, re-scan à la sortie) ou un sceau NFC inviolable sur le slab, puis les cartes brutes via le service d'authentification et de gradation à l'entrée en coffre.

## Structure

```
contracts/escrow/src/lib.rs   # le contrat d'escrow, commenté
contracts/escrow/src/test.rs  # 13 tests
contracts/vault/src/lib.rs    # le contrat coffre, commenté
contracts/vault/src/test.rs   # 11 tests
apps/agent/src/               # config, vault, photos, appraise, paywall, payer, server
apps/agent/scripts/           # x402-smoke.mjs, pay-and-appraise.mjs
demo-web-page/                # marketplace web : index.html, styles.css, app.js, data/, img/, serve.mjs, seed-market.sh
mobile-app/                   # marketplace mobile : un seul index.html
docs/                         # photos de référence de la carte, captures, schéma d'architecture (.excalidraw + .svg)
README.md                     # ce README en anglais
AGENTS.md                     # contexte pour les agents de code
```
