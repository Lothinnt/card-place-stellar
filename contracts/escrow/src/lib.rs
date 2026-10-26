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
//! - Un échange ne quitte l'état `Funded` qu'une seule fois : vers `Released` ou `Refunded`.
//! - Le montant qui sort du contrat est exactement le montant déposé.
//! - Aucune IA, aucun oracle : seules les règles écrites ici déplacent l'argent.

#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, token, Address, Env, Symbol};

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
    /// Prochain identifiant d'échange (instance storage).
    NextId,
    /// Un échange, par identifiant (persistent storage).
    Trade(u32),
}

#[contract]
pub struct CardEscrow;

#[contractimpl]
impl CardEscrow {
    /// Déploiement : fixe le dépositaire et l'actif de paiement une fois pour toutes.
    pub fn __constructor(env: Env, custodian: Address, token: Address) {
        env.storage().instance().set(&DataKey::Custodian, &custodian);
        env.storage().instance().set(&DataKey::Token, &token);
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
            status: Status::Funded,
            opened_at: now,
            deadline: now.saturating_add(deadline_ledgers),
        };
        Self::save(&env, id, &trade);
        Ok(id)
    }

    /// Carte reçue et conforme : le dépositaire libère les fonds vers le vendeur.
    pub fn release(env: Env, trade_id: u32) -> Result<(), Error> {
        Self::custodian(env.clone()).require_auth();

        let mut trade = Self::load(&env, trade_id)?;
        if trade.status != Status::Funded {
            return Err(Error::InvalidStatus);
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
        trade.status = Status::Refunded;
        Self::save(&env, trade_id, &trade);
        Ok(())
    }

    /// Lecture publique d'un échange.
    pub fn get_trade(env: Env, trade_id: u32) -> Result<Trade, Error> {
        Self::load(&env, trade_id)
    }

    // ----- helpers privés -----

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
