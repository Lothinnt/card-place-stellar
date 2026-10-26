#![cfg(test)]

use super::*;
use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger as _},
    token::{StellarAssetClient, TokenClient},
    Address, BytesN, Env, String, Symbol,
};

/// Un coffre de test : connaît une seule carte, dans l'état qu'on lui donne.
#[contract]
pub struct FakeVault;

#[contractimpl]
impl FakeVault {
    pub fn __constructor(env: Env, known: Symbol, status: CardStatus) {
        env.storage().instance().set(&Symbol::new(&env, "known"), &known);
        env.storage().instance().set(&Symbol::new(&env, "status"), &status);
    }
    pub fn get_card(env: Env, asset_code: Symbol) -> Result<Card, soroban_sdk::Error> {
        let known: Symbol = env.storage().instance().get(&Symbol::new(&env, "known")).unwrap();
        if asset_code != known {
            return Err(soroban_sdk::Error::from_contract_error(2));
        }
        let status: CardStatus = env.storage().instance().get(&Symbol::new(&env, "status")).unwrap();
        Ok(Card {
            token: env.current_contract_address(),
            grader: Symbol::new(&env, "PSA"),
            cert: Symbol::new(&env, "137798077"),
            grade: 10,
            description: String::from_str(&env, "Zekrom ex"),
            photo_hash: BytesN::from_array(&env, &[7u8; 32]),
            status,
            redeemer: None,
            tracking: None,
            registered_at: 1,
        })
    }
}

const PRICE: i128 = 943_0000000; // 943 USDC
const WEEK: u32 = 120_960; // ~7 jours de ledgers
const BUYER_FUNDS: i128 = 2_000_0000000; // 2 000 USDC

struct Setup {
    env: Env,
    vault: Address,
    custodian: Address,
    buyer: Address,
    seller: Address,
    usdc: TokenClient<'static>,
    escrow: CardEscrowClient<'static>,
    card: Symbol,
}

/// Dépositaire, acheteur avec 2 000 USDC, vendeur, un USDC de test (SAC), un coffre
/// qui tient `PSA137798077` en état `InVault`, et le contrat.
fn setup() -> Setup {
    setup_with_vault(CardStatus::InVault)
}

fn setup_with_vault(status: CardStatus) -> Setup {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| l.sequence_number = 1_000);

    let custodian = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);

    let issuer = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(issuer);
    StellarAssetClient::new(&env, &sac.address()).mint(&buyer, &BUYER_FUNDS);

    let card = Symbol::new(&env, "PSA137798077");
    let vault = env.register(FakeVault, (&card, status));
    let id = env.register(CardEscrow, (&custodian, &sac.address(), &vault));
    Setup {
        env: env.clone(),
        vault,
        custodian,
        buyer,
        seller,
        usdc: TokenClient::new(&env, &sac.address()),
        escrow: CardEscrowClient::new(&env, &id),
        card,
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
    assert_eq!(s.escrow.vault(), s.vault);
}

#[test]
fn release_refuses_a_card_unknown_to_the_vault() {
    let s = setup();
    let id = s.escrow.open(&s.buyer, &s.seller, &Symbol::new(&s.env, "PSA000000000"), &PRICE, &WEEK);
    assert_eq!(s.escrow.try_release(&id), Err(Ok(Error::CardNotInVault)));
    // l'argent n'a pas bougé
    assert_eq!(s.usdc.balance(&s.escrow.address), PRICE);
    assert_eq!(s.usdc.balance(&s.seller), 0);
    assert_eq!(s.escrow.get_trade(&id).status, Status::Funded);
}

#[test]
fn release_refuses_a_card_that_left_the_vault() {
    let s = setup_with_vault(CardStatus::Shipped);
    let id = open(&s);
    assert_eq!(s.escrow.try_release(&id), Err(Ok(Error::CardNotInVault)));
    assert_eq!(s.usdc.balance(&s.seller), 0);
    // le dépositaire peut toujours rembourser l'acheteur
    s.escrow.refund(&s.custodian, &id);
    assert_eq!(s.usdc.balance(&s.buyer), BUYER_FUNDS);
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
fn settle_splits_between_buyer_and_seller() {
    let s = setup();
    let id = open(&s);
    let penalty: i128 = 50_0000000; // 50 USDC back to the buyer: the card arrived late
    s.escrow.settle(&id, &penalty);
    assert_eq!(s.usdc.balance(&s.escrow.address), 0);
    assert_eq!(s.usdc.balance(&s.buyer), BUYER_FUNDS - PRICE + penalty);
    assert_eq!(s.usdc.balance(&s.seller), PRICE - penalty);
    let t = s.escrow.get_trade(&id);
    assert_eq!(t.status, Status::Settled);
    assert_eq!(t.refunded, penalty);
    // closed for good
    assert_eq!(s.escrow.try_release(&id), Err(Ok(Error::InvalidStatus)));
    assert_eq!(s.escrow.try_refund(&s.custodian, &id), Err(Ok(Error::InvalidStatus)));
}

#[test]
fn settle_rejects_a_bad_split_and_a_card_not_in_vault() {
    let s = setup();
    let id = open(&s);
    assert_eq!(s.escrow.try_settle(&id, &(PRICE + 1)), Err(Ok(Error::InvalidSplit)));
    assert_eq!(s.escrow.try_settle(&id, &-1), Err(Ok(Error::InvalidSplit)));
    assert_eq!(s.escrow.get_trade(&id).status, Status::Funded);

    let s2 = setup_with_vault(CardStatus::Shipped);
    let id2 = open(&s2);
    assert_eq!(s2.escrow.try_settle(&id2, &1_0000000), Err(Ok(Error::CardNotInVault)));
    assert_eq!(s2.usdc.balance(&s2.escrow.address), PRICE);
}

#[test]
fn settle_requires_custodian_auth() {
    let s = setup();
    let id = open(&s);
    s.escrow.settle(&id, &1_0000000);
    assert_eq!(s.env.auths()[0].0, s.custodian);
}

#[test]
fn unknown_trade_fails() {
    let s = setup();
    assert_eq!(s.escrow.try_get_trade(&42), Err(Ok(Error::TradeNotFound)));
    assert_eq!(s.escrow.try_release(&42), Err(Ok(Error::TradeNotFound)));
}
