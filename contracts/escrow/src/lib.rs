//! L'escrow de Card Place — l'argent de l'acheteur est bloqué dans le contrat
//! jusqu'à ce que la carte soit arrivée et vérifiée par le dépositaire.
//!
//! Modèle :
//! - L'acheteur ouvre un échange en déposant le prix en USDC (ou tout actif SEP-41)
//!   dans le contrat. Ni le vendeur ni Card Place ne détiennent cet argent.
//! - Le vendeur n'expédie la carte qu'une fois le dépôt visible on-chain.
//! - Le dépositaire reçoit la carte, l'inspecte, et choisit : `release` (la carte est
//!   conforme, le contrat paie le vendeur) ou `refund` (la carte ne l'est pas, le
//!   contrat rembourse l'acheteur).
//! - `release` ne paie que si le contrat coffre confirme que la carte est bien entrée
//!   en coffre (`InVault`) : l'argent ne sort jamais pour une carte qui n'est pas là.
//! - Entre les deux, `settle` : la carte est conforme mais le vendeur a manqué à une
//!   obligation (retard d'envoi, par exemple). Le dépositaire rend une partie du montant
//!   à l'acheteur et verse le reste au vendeur, en une seule transaction.
//! - Si rien ne se passe avant l'échéance, l'acheteur peut se rembourser lui-même :
//!   son argent ne reste jamais bloqué.
//!
//! Rôles :
//! - `custodian` : le dépositaire (Card Place). Seul à pouvoir libérer ou rembourser
//!   avant l'échéance.
//! - `buyer` : dépose les fonds ; peut se rembourser après l'échéance.
//! - `seller` : reçoit les fonds à la libération.
//!
//! Invariants :
//! - Un échange ne quitte l'état `Funded` qu'une seule fois : vers `Released`, `Settled` ou `Refunded`.
//! - `release` exige que le coffre connaisse la carte et la tienne en état `InVault`.
//! - Le montant qui sort du contrat est exactement le montant déposé.
//! - Aucune IA, aucun oracle : seules les règles écrites ici déplacent l'argent.

#![no_std]

use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, token, Address, BytesN,
    Env, String, Symbol,
};

/// Ce que l'escrow lit dans le contrat coffre (`contracts/vault`) : la fiche d'une carte.
/// Les types doivent rester identiques à ceux du coffre.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardStatus {
    InVault = 0,
    RedeemRequested = 1,
    Shipped = 2,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Card {
    pub token: Address,
    pub grader: Symbol,
    pub cert: Symbol,
    pub grade: u32,
    pub description: String,
    pub photo_hash: BytesN<32>,
    pub status: CardStatus,
    pub redeemer: Option<Address>,
    pub tracking: Option<String>,
    pub registered_at: u32,
}

#[contractclient(name = "VaultClient")]
pub trait VaultInterface {
    fn get_card(env: Env, asset_code: Symbol) -> Result<Card, soroban_sdk::Error>;
}

/// Durée de vie du stockage persistant, en ledgers (~5 s / ledger).
const TTL_THRESHOLD: u32 = 518_400; // 30 jours
const TTL_EXTEND_TO: u32 = 6_312_000; // 365 jours

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Aucun échange sous cet identifiant.
    TradeNotFound = 1,
    /// L'échange n'est plus dans l'état `Funded`.
    InvalidStatus = 2,
    /// Le montant doit être strictement positif.
    InvalidAmount = 3,
    /// L'échéance n'est pas encore passée : seul le dépositaire peut rembourser.
    NotExpired = 4,
    /// L'acheteur et le vendeur doivent être deux adresses distinctes.
    SameParty = 5,
    /// Le coffre ne tient pas cette carte en état `InVault` : rien n'est payé.
    CardNotInVault = 6,
    /// La part remboursée doit être comprise entre 0 et le montant déposé.
    InvalidSplit = 7,
}

/// État d'un échange.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// Les fonds de l'acheteur sont dans le contrat ; le vendeur peut expédier.
    Funded = 0,
    /// Carte conforme : les fonds ont été versés au vendeur.
    Released = 1,
    /// Carte non conforme ou échéance passée : les fonds sont revenus à l'acheteur.
    Refunded = 2,
    /// Carte conforme mais pénalité : une part à l'acheteur, le reste au vendeur.
    Settled = 3,
}

/// Un échange sous escrow.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    pub buyer: Address,
    pub seller: Address,
    /// Code de l'actif qui représente la carte (ex. `PSA137798077`), pour le lien avec le coffre.
    pub card: Symbol,
    /// Montant déposé, en unités de base de l'actif de paiement (7 décimales).
    pub amount: i128,
    /// Part rendue à l'acheteur (0 sauf `Refunded` ou `Settled`).
    pub refunded: i128,
    pub status: Status,
    /// Ledger d'ouverture.
    pub opened_at: u32,
    /// Ledger à partir duquel l'acheteur peut se rembourser seul.
    pub deadline: u32,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Adresse du dépositaire (instance storage).
    Custodian,
    /// Adresse du Stellar Asset Contract de l'actif de paiement, ex. USDC (instance storage).
    Token,
    /// Adresse du contrat coffre, consulté avant de payer le vendeur (instance storage).
    Vault,
    /// Prochain identifiant d'échange (instance storage).
    NextId,
    /// Un échange, par identifiant (persistent storage).
    Trade(u32),
}

#[contract]
pub struct CardEscrow;

#[contractimpl]
impl CardEscrow {
    /// Déploiement : fixe le dépositaire, l'actif de paiement et le contrat coffre.
    pub fn __constructor(env: Env, custodian: Address, token: Address, vault: Address) {
        env.storage().instance().set(&DataKey::Custodian, &custodian);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage().instance().set(&DataKey::Vault, &vault);
        env.storage().instance().set(&DataKey::NextId, &1u32);
    }

    /// Adresse du dépositaire.
    pub fn custodian(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Custodian).expect("custodian not set")
    }

    /// Adresse du contrat de l'actif de paiement.
    pub fn token(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Token).expect("token not set")
    }

    /// Adresse du contrat coffre.
    pub fn vault(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Vault).expect("vault not set")
    }

    /// L'acheteur ouvre un échange : `amount` est transféré de son compte vers le contrat.
    ///
    /// `deadline_ledgers` : nombre de ledgers avant que l'acheteur puisse se rembourser
    /// seul (ex. 120 960 ≈ 7 jours). Renvoie l'identifiant de l'échange.
    pub fn open(
        env: Env,
        buyer: Address,
        seller: Address,
        card: Symbol,
        amount: i128,
        deadline_ledgers: u32,
    ) -> Result<u32, Error> {
        buyer.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if buyer == seller {
            return Err(Error::SameParty);
        }

        // Les fonds entrent dans le contrat. L'autorisation de l'acheteur couvre ce transfert.
        token::Client::new(&env, &Self::token(env.clone())).transfer(
            &buyer,
            &env.current_contract_address(),
            &amount,
        );

        let id: u32 = env.storage().instance().get(&DataKey::NextId).unwrap_or(1);
        env.storage().instance().set(&DataKey::NextId, &(id + 1));

        let now = env.ledger().sequence();
        let trade = Trade {
            buyer,
            seller,
            card,
            amount,
            refunded: 0,
            status: Status::Funded,
            opened_at: now,
            deadline: now.saturating_add(deadline_ledgers),
        };
        Self::save(&env, id, &trade);
        Ok(id)
    }

    /// Carte reçue et conforme : le dépositaire libère les fonds vers le vendeur.
    ///
    /// Le contrat vérifie d'abord auprès du coffre que la carte y est bien entrée
    /// (`InVault`). Sans cette fiche, l'appel échoue et l'argent reste bloqué.
    pub fn release(env: Env, trade_id: u32) -> Result<(), Error> {
        Self::custodian(env.clone()).require_auth();

        let mut trade = Self::load(&env, trade_id)?;
        if trade.status != Status::Funded {
            return Err(Error::InvalidStatus);
        }

        if !Self::card_in_vault(&env, &trade.card) {
            return Err(Error::CardNotInVault);
        }

        token::Client::new(&env, &Self::token(env.clone())).transfer(
            &env.current_contract_address(),
            &trade.seller,
            &trade.amount,
        );
        trade.status = Status::Released;
        Self::save(&env, trade_id, &trade);
        Ok(())
    }

    /// Les fonds reviennent à l'acheteur.
    ///
    /// Avant l'échéance : réservé au dépositaire (carte non conforme, jamais arrivée…).
    /// Après l'échéance : l'acheteur peut l'appeler lui-même, sans dépendre de personne.
    pub fn refund(env: Env, caller: Address, trade_id: u32) -> Result<(), Error> {
        caller.require_auth();

        let mut trade = Self::load(&env, trade_id)?;
        if trade.status != Status::Funded {
            return Err(Error::InvalidStatus);
        }
        let is_custodian = caller == Self::custodian(env.clone());
        let expired = env.ledger().sequence() >= trade.deadline;
        if !is_custodian && !(caller == trade.buyer && expired) {
            return Err(Error::NotExpired);
        }

        token::Client::new(&env, &Self::token(env.clone())).transfer(
            &env.current_contract_address(),
            &trade.buyer,
            &trade.amount,
        );
        trade.refunded = trade.amount;
        trade.status = Status::Refunded;
        Self::save(&env, trade_id, &trade);
        Ok(())
    }

    /// Carte conforme, mais le vendeur a manqué à une obligation (retard d'envoi…) :
    /// le dépositaire rend `refund_amount` à l'acheteur et verse le reste au vendeur.
    ///
    /// Comme `release`, exige que le coffre tienne la carte (`InVault`).
    /// `refund_amount` = 0 équivaut à `release`, = `amount` équivaut à `refund`.
    pub fn settle(env: Env, trade_id: u32, refund_amount: i128) -> Result<(), Error> {
        Self::custodian(env.clone()).require_auth();

        let mut trade = Self::load(&env, trade_id)?;
        if trade.status != Status::Funded {
            return Err(Error::InvalidStatus);
        }
        if refund_amount < 0 || refund_amount > trade.amount {
            return Err(Error::InvalidSplit);
        }
        if !Self::card_in_vault(&env, &trade.card) {
            return Err(Error::CardNotInVault);
        }

        let token = token::Client::new(&env, &Self::token(env.clone()));
        let me = env.current_contract_address();
        if refund_amount > 0 {
            token.transfer(&me, &trade.buyer, &refund_amount);
        }
        let to_seller = trade.amount - refund_amount;
        if to_seller > 0 {
            token.transfer(&me, &trade.seller, &to_seller);
        }
        trade.refunded = refund_amount;
        trade.status = Status::Settled;
        Self::save(&env, trade_id, &trade);
        Ok(())
    }

    /// Lecture publique d'un échange.
    pub fn get_trade(env: Env, trade_id: u32) -> Result<Trade, Error> {
        Self::load(&env, trade_id)
    }

    // ----- helpers privés -----

    /// Demande au coffre si la carte y est bien, en état `InVault`.
    fn card_in_vault(env: &Env, card: &Symbol) -> bool {
        VaultClient::new(env, &Self::vault(env.clone()))
            .try_get_card(card)
            .map(|r| r.map(|c| c.status == CardStatus::InVault).unwrap_or(false))
            .unwrap_or(false)
    }

    fn load(env: &Env, id: u32) -> Result<Trade, Error> {
        env.storage().persistent().get(&DataKey::Trade(id)).ok_or(Error::TradeNotFound)
    }

    fn save(env: &Env, id: u32, trade: &Trade) {
        let key = DataKey::Trade(id);
        env.storage().persistent().set(&key, trade);
        env.storage().persistent().extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}

mod test;
