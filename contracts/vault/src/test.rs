#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token::{StellarAssetClient, TokenClient},
    Address, BytesN, Env, String, Symbol,
};

/// Monte un environnement complet : dépositaire (= émetteur), un détenteur,
/// un actif natif wrappé en SAC avec 1 unité chez le détenteur, et le contrat.
struct Setup {
    env: Env,
    custodian: Address,
    holder: Address,
    token: Address,
    vault: CardVaultClient<'static>,
    asset_code: Symbol,
    photo_hash: BytesN<32>,
}

fn setup() -> Setup {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.sequence_number = 1_000);

    let custodian = Address::generate(&env);
    let holder = Address::generate(&env);

    // Actif natif émis par le dépositaire, wrappé par son Stellar Asset Contract.
    let sac = env.register_stellar_asset_contract_v2(custodian.clone());
    let token = sac.address();
    StellarAssetClient::new(&env, &token).mint(&holder, &ONE_CARD);

    let vault_id = env.register(CardVault, (&custodian,));
    let vault = CardVaultClient::new(&env, &vault_id);

    Setup {
        env: env.clone(),
        custodian,
        holder,
        token,
        vault,
        asset_code: Symbol::new(&env, "PSA137798077"),
        photo_hash: BytesN::from_array(&env, &[7u8; 32]),
    }
}

fn register(s: &Setup) {
    s.vault.register_card(
        &s.asset_code,
        &s.token,
        &Symbol::new(&s.env, "PSA"),
        &Symbol::new(&s.env, "137798077"),
        &10,
        &String::from_str(&s.env, "2025 Pokemon BLK FR Zekrom ex #166 SIR"),
        &s.photo_hash,
    );
}

#[test]
fn constructor_sets_custodian() {
    let s = setup();
    assert_eq!(s.vault.custodian(), s.custodian);
}

#[test]
fn register_then_read() {
    let s = setup();
    register(&s);

    let card = s.vault.get_card(&s.asset_code);
    assert_eq!(card.token, s.token);
    assert_eq!(card.grader, Symbol::new(&s.env, "PSA"));
    assert_eq!(card.cert, Symbol::new(&s.env, "137798077"));
    assert_eq!(card.grade, 10);
    assert_eq!(card.photo_hash, s.photo_hash);
    assert_eq!(card.status, Status::InVault);
    assert_eq!(card.redeemer, None);
    assert_eq!(card.tracking, None);
    assert_eq!(card.registered_at, 1_000);
}

#[test]
fn register_requires_custodian_auth() {
    let s = setup();
    register(&s);
    // La dernière autorisation enregistrée doit venir du dépositaire.
    let auths = s.env.auths();
    assert_eq!(auths[0].0, s.custodian);
}

#[test]
fn register_twice_fails() {
    let s = setup();
    register(&s);
    let res = s.vault.try_register_card(
        &s.asset_code,
        &s.token,
        &Symbol::new(&s.env, "PSA"),
        &Symbol::new(&s.env, "137798077"),
        &10,
        &String::from_str(&s.env, "dup"),
        &s.photo_hash,
    );
    assert_eq!(res, Err(Ok(Error::CardAlreadyRegistered)));
}

#[test]
fn get_unknown_card_fails() {
    let s = setup();
    let res = s.vault.try_get_card(&Symbol::new(&s.env, "NOPE"));
    assert_eq!(res, Err(Ok(Error::CardNotFound)));
}

#[test]
fn redeem_moves_token_and_changes_status() {
    let s = setup();
    register(&s);
    let token = TokenClient::new(&s.env, &s.token);
    assert_eq!(token.balance(&s.holder), ONE_CARD);

    s.vault.redeem(&s.holder, &s.asset_code);

    // Le jeton est passé du détenteur au dépositaire.
    // (Sur le réseau, si le dépositaire est aussi l'émetteur de l'actif,
    // ce transfert équivaut à un burn. Dans l'env de test, l'émetteur du SAC
    // est une clé distincte, donc le solde du dépositaire est visible.)
    assert_eq!(token.balance(&s.holder), 0);
    assert_eq!(token.balance(&s.custodian), ONE_CARD);

    let card = s.vault.get_card(&s.asset_code);
    assert_eq!(card.status, Status::RedeemRequested);
    assert_eq!(card.redeemer, Some(s.holder.clone()));
}

#[test]
fn redeem_requires_holder_auth() {
    let s = setup();
    register(&s);
    s.vault.redeem(&s.holder, &s.asset_code);
    let auths = s.env.auths();
    assert_eq!(auths[0].0, s.holder);
}

#[test]
fn redeem_twice_fails() {
    let s = setup();
    register(&s);
    s.vault.redeem(&s.holder, &s.asset_code);
    let res = s.vault.try_redeem(&s.holder, &s.asset_code);
    assert_eq!(res, Err(Ok(Error::InvalidStatus)));
}

#[test]
fn redeem_without_token_fails() {
    let s = setup();
    register(&s);
    let stranger = Address::generate(&s.env);
    // Pas de jeton : le transfert SAC échoue, l'appel échoue, rien n'est modifié.
    let res = s.vault.try_redeem(&stranger, &s.asset_code);
    assert!(res.is_err());
    assert_eq!(s.vault.get_card(&s.asset_code).status, Status::InVault);
}

#[test]
fn mark_shipped_full_lifecycle() {
    let s = setup();
    register(&s);
    s.vault.redeem(&s.holder, &s.asset_code);

    let tracking = String::from_str(&s.env, "LA123456789FR");
    s.vault.mark_shipped(&s.asset_code, &tracking);

    let card = s.vault.get_card(&s.asset_code);
    assert_eq!(card.status, Status::Shipped);
    assert_eq!(card.tracking, Some(tracking));
}

#[test]
fn mark_shipped_before_redeem_fails() {
    let s = setup();
    register(&s);
    let res = s
        .vault
        .try_mark_shipped(&s.asset_code, &String::from_str(&s.env, "X"));
    assert_eq!(res, Err(Ok(Error::InvalidStatus)));
}
