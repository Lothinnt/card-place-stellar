#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token::{StellarAssetClient, TokenClient},
    Address, Env, Symbol,
};

const PRICE: i128 = 943_0000000; // 943 USDC
const WEEK: u32 = 120_960; // ~7 jours de ledgers
const BUYER_FUNDS: i128 = 2_000_0000000; // 2 000 USDC

struct Setup {
    env: Env,
    custodian: Address,
    buyer: Address,
    seller: Address,
    usdc: TokenClient<'static>,
    escrow: CardEscrowClient<'static>,
    card: Symbol,
}

/// Dépositaire, acheteur avec 2 000 USDC, vendeur, un USDC de test (SAC) et le contrat.
fn setup() -> Setup {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.sequence_number = 1_000);

    let custodian = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);

    let issuer = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(issuer);
    StellarAssetClient::new(&env, &sac.address()).mint(&buyer, &BUYER_FUNDS);

    let id = env.register(CardEscrow, (&custodian, &sac.address()));
    Setup {
        env: env.clone(),
        custodian,
        buyer,
        seller,
        usdc: TokenClient::new(&env, &sac.address()),
        escrow: CardEscrowClient::new(&env, &id),
        card: Symbol::new(&env, "PSA137798077"),
    }
}

fn open(s: &Setup) -> u32 {
    s.escrow.open(&s.buyer, &s.seller, &s.card, &PRICE, &WEEK)
}

#[test]
fn constructor_sets_roles() {
    let s = setup();
    assert_eq!(s.escrow.custodian(), s.custodian);
    assert_eq!(s.escrow.token(), s.usdc.address);
}

#[test]
fn open_locks_funds_in_contract() {
    let s = setup();
    let id = open(&s);
    assert_eq!(id, 1);
    assert_eq!(s.usdc.balance(&s.buyer), BUYER_FUNDS - PRICE);
    assert_eq!(s.usdc.balance(&s.escrow.address), PRICE);
    assert_eq!(s.usdc.balance(&s.seller), 0);

    let t = s.escrow.get_trade(&id);
    assert_eq!(t.buyer, s.buyer);
    assert_eq!(t.seller, s.seller);
    assert_eq!(t.card, s.card);
    assert_eq!(t.amount, PRICE);
    assert_eq!(t.status, Status::Funded);
    assert_eq!(t.opened_at, 1_000);
    assert_eq!(t.deadline, 1_000 + WEEK);
}

#[test]
fn open_requires_buyer_auth() {
    let s = setup();
    open(&s);
    assert_eq!(s.env.auths()[0].0, s.buyer);
}

#[test]
fn ids_increment() {
    let s = setup();
    assert_eq!(open(&s), 1);
    assert_eq!(open(&s), 2);
}

#[test]
fn open_rejects_zero_amount_and_same_party() {
    let s = setup();
    assert_eq!(
        s.escrow.try_open(&s.buyer, &s.seller, &s.card, &0, &WEEK),
        Err(Ok(Error::InvalidAmount))
    );
    assert_eq!(
        s.escrow.try_open(&s.buyer, &s.buyer, &s.card, &PRICE, &WEEK),
        Err(Ok(Error::SameParty))
    );
}

#[test]
fn release_pays_the_seller() {
    let s = setup();
    let id = open(&s);
    s.escrow.release(&id);
    assert_eq!(s.usdc.balance(&s.escrow.address), 0);
    assert_eq!(s.usdc.balance(&s.seller), PRICE);
    assert_eq!(s.escrow.get_trade(&id).status, Status::Released);
}

#[test]
fn release_requires_custodian_auth() {
    let s = setup();
    let id = open(&s);
    s.escrow.release(&id);
    assert_eq!(s.env.auths()[0].0, s.custodian);
}

#[test]
fn release_twice_fails() {
    let s = setup();
    let id = open(&s);
    s.escrow.release(&id);
    assert_eq!(s.escrow.try_release(&id), Err(Ok(Error::InvalidStatus)));
    assert_eq!(s.escrow.try_refund(&s.custodian, &id), Err(Ok(Error::InvalidStatus)));
}

#[test]
fn custodian_can_refund_anytime() {
    let s = setup();
    let id = open(&s);
    s.escrow.refund(&s.custodian, &id);
    assert_eq!(s.usdc.balance(&s.buyer), BUYER_FUNDS);
    assert_eq!(s.usdc.balance(&s.escrow.address), 0);
    assert_eq!(s.escrow.get_trade(&id).status, Status::Refunded);
}

#[test]
fn buyer_cannot_refund_before_deadline() {
    let s = setup();
    let id = open(&s);
    assert_eq!(s.escrow.try_refund(&s.buyer, &id), Err(Ok(Error::NotExpired)));
    assert_eq!(s.escrow.get_trade(&id).status, Status::Funded);
}

#[test]
fn buyer_can_refund_after_deadline() {
    let s = setup();
    let id = open(&s);
    s.env.ledger().with_mut(|l| l.sequence_number = 1_000 + WEEK);
    s.escrow.refund(&s.buyer, &id);
    assert_eq!(s.usdc.balance(&s.buyer), BUYER_FUNDS);
    assert_eq!(s.escrow.get_trade(&id).status, Status::Refunded);
}

#[test]
fn seller_can_never_refund() {
    let s = setup();
    let id = open(&s);
    s.env.ledger().with_mut(|l| l.sequence_number = 1_000 + WEEK);
    assert_eq!(s.escrow.try_refund(&s.seller, &id), Err(Ok(Error::NotExpired)));
}

#[test]
fn unknown_trade_fails() {
    let s = setup();
    assert_eq!(s.escrow.try_get_trade(&42), Err(Ok(Error::TradeNotFound)));
    assert_eq!(s.escrow.try_release(&42), Err(Ok(Error::TradeNotFound)));
}
