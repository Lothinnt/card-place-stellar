//! Le coffre de Card Place — registre on-chain de cartes gradées conservées en coffre.
//!
//! Modèle :
//! - Chaque carte physique est représentée par **un actif Stellar natif** (code = certificat,
//!   1 unité émise). L'échange passe par le carnet d'ordres natif, pas par ce contrat.
//! - Ce contrat ne fait que ce que le protocole ne sait pas faire : lier le jeton au
//!   certificat du gradeur et au hash de la photo, tenir l'état physique de la carte,
//!   et gérer la sortie du coffre (`redeem`) en récupérant le jeton via le
//!   Stellar Asset Contract (SAC) de l'actif.
//!
//! Rôles :
//! - `custodian` : le dépositaire (GradedCardShop). Seul à pouvoir enregistrer une carte
//!   et la marquer expédiée.
//! - `holder` : le détenteur du jeton. Seul à pouvoir demander la sortie physique.
//!
//! Invariants :
//! - Une carte n'est enregistrée qu'une fois (clé = code d'actif).
//! - Les transitions d'état sont strictement InVault -> RedeemRequested -> Shipped.
//! - `redeem` transfère exactement 1 unité du jeton du détenteur vers le dépositaire.
//!   Si le dépositaire est aussi l'émetteur de l'actif, ce transfert brûle le jeton.

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, BytesN, Env, String,
    Symbol,
};

/// 1 unité d'un actif Stellar natif (7 décimales). Une carte = une unité.
pub const ONE_CARD: i128 = 10_000_000;

/// Durée de vie du stockage persistant, en ledgers (~5 s / ledger).
/// Seuil : on prolonge quand il reste moins de ~30 jours. Cible : ~1 an.
const TTL_THRESHOLD: u32 = 518_400; // 30 jours
const TTL_EXTEND_TO: u32 = 6_312_000; // 365 jours

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// La carte est déjà enregistrée sous ce code d'actif.
    CardAlreadyRegistered = 1,
    /// Aucune carte sous ce code d'actif.
    CardNotFound = 2,
    /// La transition demandée n'est pas permise depuis l'état courant.
    InvalidStatus = 3,
}

/// État physique de la carte.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// En coffre chez le dépositaire ; le jeton circule librement.
    InVault = 0,
    /// Le détenteur a rendu le jeton et demandé l'expédition.
    RedeemRequested = 1,
    /// Le dépositaire a expédié la carte physique.
    Shipped = 2,
}

/// Fiche d'une carte en coffre.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Card {
    /// Adresse du Stellar Asset Contract du jeton qui représente la carte.
    pub token: Address,
    /// Gradeur (ex. `PSA`, `PCA`, `BGS`).
    pub grader: Symbol,
    /// Numéro de certificat du gradeur (ex. `137798077`).
    pub cert: Symbol,
    /// Note attribuée, sur 10 (ex. 10 pour GEM MINT 10).
    pub grade: u32,
    /// Description lisible (ex. "2025 Pokemon BLK FR Zekrom ex #166 SIR").
    pub description: String,
    /// SHA-256 de la photo de référence de la carte (recto).
    pub photo_hash: BytesN<32>,
    /// État physique courant.
    pub status: Status,
    /// Détenteur ayant demandé la sortie, le cas échéant.
    pub redeemer: Option<Address>,
    /// Numéro de suivi de l'expédition, le cas échéant.
    pub tracking: Option<String>,
    /// Ledger d'enregistrement.
    pub registered_at: u32,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Adresse du dépositaire (instance storage).
    Custodian,
    /// Fiche d'une carte, par code d'actif (persistent storage).
    Card(Symbol),
}

#[contract]
pub struct CardVault;

#[contractimpl]
impl CardVault {
    /// Déploiement : fixe le dépositaire une fois pour toutes.
    pub fn __constructor(env: Env, custodian: Address) {
        env.storage().instance().set(&DataKey::Custodian, &custodian);
    }

    /// Adresse du dépositaire.
    pub fn custodian(env: Env) -> Address {
        env.storage()
            .instance()
            .get(&DataKey::Custodian)
            .expect("custodian not set")
    }

    /// Enregistre une carte entrée en coffre. Réservé au dépositaire.
    ///
    /// `asset_code` est le code de l'actif Stellar qui représente la carte
    /// (ex. `PSA137798077`) ; `token` est l'adresse de son Stellar Asset Contract.
    pub fn register_card(
        env: Env,
        asset_code: Symbol,
        token: Address,
        grader: Symbol,
        cert: Symbol,
        grade: u32,
        description: String,
        photo_hash: BytesN<32>,
    ) -> Result<(), Error> {
        Self::custodian(env.clone()).require_auth();

        let key = DataKey::Card(asset_code);
        if env.storage().persistent().has(&key) {
            return Err(Error::CardAlreadyRegistered);
        }

        let card = Card {
            token,
            grader,
            cert,
            grade,
            description,
            photo_hash,
            status: Status::InVault,
            redeemer: None,
            tracking: None,
            registered_at: env.ledger().sequence(),
        };
        Self::save(&env, &key, &card);
        Ok(())
    }

    /// Le détenteur rend le jeton et demande l'expédition de la carte physique.
    ///
    /// Transfère 1 unité du jeton de `holder` vers le dépositaire via le SAC.
    /// `holder` doit autoriser l'appel (et donc le transfert).
    pub fn redeem(env: Env, holder: Address, asset_code: Symbol) -> Result<(), Error> {
        holder.require_auth();

        let key = DataKey::Card(asset_code);
        let mut card = Self::load(&env, &key)?;
        if card.status != Status::InVault {
            return Err(Error::InvalidStatus);
        }

        let custodian = Self::custodian(env.clone());
        token::Client::new(&env, &card.token).transfer(&holder, &custodian, &ONE_CARD);

        card.status = Status::RedeemRequested;
        card.redeemer = Some(holder);
        Self::save(&env, &key, &card);
        Ok(())
    }

    /// Le dépositaire confirme l'expédition avec un numéro de suivi.
    pub fn mark_shipped(env: Env, asset_code: Symbol, tracking: String) -> Result<(), Error> {
        Self::custodian(env.clone()).require_auth();

        let key = DataKey::Card(asset_code);
        let mut card = Self::load(&env, &key)?;
        if card.status != Status::RedeemRequested {
            return Err(Error::InvalidStatus);
        }

        card.status = Status::Shipped;
        card.tracking = Some(tracking);
        Self::save(&env, &key, &card);
        Ok(())
    }

    /// Lecture publique de la fiche d'une carte.
    pub fn get_card(env: Env, asset_code: Symbol) -> Result<Card, Error> {
        Self::load(&env, &DataKey::Card(asset_code))
    }

    // ----- helpers privés -----

    fn load(env: &Env, key: &DataKey) -> Result<Card, Error> {
        env.storage()
            .persistent()
            .get(key)
            .ok_or(Error::CardNotFound)
    }

    fn save(env: &Env, key: &DataKey, card: &Card) {
        env.storage().persistent().set(key, card);
        env.storage()
            .persistent()
            .extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}

mod test;
