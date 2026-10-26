#!/usr/bin/env bash
# Live demo on Stellar testnet: the escrow refuses to pay for a card the vault doesn't hold,
# and pays the seller for a card it does. Every step prints its stellar.expert link and opens it.
#
#   bash demo/demo.sh                            # needs the identities moi / collectionneur / vendeur and the aliases vault / escrow
#   OPEN_LINKS=0 bash demo/demo.sh               # keep the explorer links in the terminal, don't open a browser
#   PRICE_USDC=9 TRAP_USDC=1 bash demo/demo.sh   # smaller amounts if the buyer has little testnet USDC (default 943 + 50)
#
# Identities: moi = Card Place (custodian), collectionneur = buyer, vendeur = seller.
# Aliases: created by `stellar contract deploy --alias vault|escrow` (README, Reproduce §1). If they are missing and
# `moi` is Card Place's custodian key, the script points them at the contracts deployed on testnet.
# This script needs bash (not sh/dash): `bash demo/demo.sh` or `./demo/demo.sh`.
set -u
cd "$(dirname "$0")/.."
export STELLAR_NETWORK=testnet

# Contracts deployed by Card Place on testnet (README, "Deployed on testnet")
VAULT_ID=CDN5OOWTOYKEXMH5CDG7APQRQEHGFRKWQEAHQR5XR3643YOM2YEMEMRO
ESCROW_ID=CC2NP5O5E5L4463WLWTEPXOR3KEBYY7FMQMZT56TYLBJUVCP3DJHLG2O
CUSTODIAN=GBNZP4YND7GXOM26YNBOAXIMTEMKNHDX7CQ5VZOJ3VKW7HDJSQR3XK75

B='\033[1m'; D='\033[2m'; G='\033[32m'; R='\033[31m'; C='\033[36m'; N='\033[0m'
OPEN_LINKS=${OPEN_LINKS:-1}
PRICE_USDC=${PRICE_USDC:-943}; TRAP_USDC=${TRAP_USDC:-50}
PRICE_UNITS=$((PRICE_USDC * 10000000)); TRAP_UNITS=$((TRAP_USDC * 10000000))   # 7 decimals

say()  { printf "\n${B}%s${N}\n" "$*"; }
note() { printf "${D}%s${N}\n" "$*"; }
fail() { printf "${R}%s${N}\n" "$*" >&2; exit 1; }
open_link() { # macOS `open`, Linux `xdg-open`, otherwise nothing
  if command -v open >/dev/null 2>&1; then open "$1"; elif command -v xdg-open >/dev/null 2>&1; then xdg-open "$1" >/dev/null 2>&1; fi
}
run()  { # prints the command with a typing effect, then runs it and shows the explorer link
  printf "${C}$ ${N}"; for ((i = 0; i < ${#1}; i++)); do printf '%s' "${1:$i:1}"; sleep 0.012; done; printf '\n'
  out=$(eval "$1" 2>&1); code=$?
  printf '%s\n' "$out" | grep -vE "^(ℹ️|📅|⚠️)" | grep -v "Simulation identified" | sed 's/^/  /'
  link=$(printf '%s\n' "$out" | grep -o 'https://stellar.expert[^ ]*' | head -1)
  if [ -n "$link" ] && [ "$OPEN_LINKS" = 1 ]; then
    open_link "$link"; sleep 6
    command -v osascript >/dev/null 2>&1 && osascript -e 'tell application "Terminal" to activate' >/dev/null 2>&1
  fi
  return $code
}
usdc() { curl -s "https://horizon-testnet.stellar.org/accounts/$1" | python3 -c "import sys,json; print(next((b['balance'] for b in json.load(sys.stdin).get('balances',[]) if b.get('asset_code')=='USDC'),'0 (no USDC trustline)'))"; }

# --- preflight: identities and contract aliases -----------------------------------------------
for who in moi collectionneur vendeur; do
  stellar keys address "$who" >/dev/null 2>&1 || fail "Missing identity '$who'. Create it: stellar keys generate $who --network testnet --fund"
done
MOI=$(stellar keys address moi); COL=$(stellar keys address collectionneur); VEN=$(stellar keys address vendeur)

for alias in vault escrow; do
  if ! stellar contract alias show "$alias" >/dev/null 2>&1; then
    if [ "$MOI" = "$CUSTODIAN" ]; then
      id=$VAULT_ID; [ "$alias" = escrow ] && id=$ESCROW_ID
      stellar contract alias add "$alias" --id "$id" >/dev/null 2>&1 || fail "Could not add the alias '$alias'"
      note "alias '$alias' → $id (Card Place's contract on testnet)"
    else
      fail "No contract alias '$alias' for testnet. The contracts Card Place deployed only accept our custodian key,
so deploy your own copy first (README, Reproduce §1): the two 'stellar contract deploy --alias vault|escrow'
commands create the aliases. To only read ours: stellar contract alias add $alias --id $([ "$alias" = vault ] && echo "$VAULT_ID" || echo "$ESCROW_ID") --network testnet"
    fi
  fi
done
VAULT=$(stellar contract alias show vault 2>/dev/null); ESCROW=$(stellar contract alias show escrow 2>/dev/null)

[ -t 1 ] && clear
say "Card Place — live on Stellar testnet"
note "custodian  $MOI"
note "buyer      $COL"
note "seller     $VEN"
note "vault      $VAULT"
note "escrow     $ESCROW"
sleep 2

say "1. The contracts: vault + escrow, 26 tests"
run "cargo test 2>&1 | grep -E '^test result'"
sleep 1

say "2. A real card from our stock enters the vault: Lugia V PSA 10, cert 86768374"
if stellar contract invoke --id vault --source-account moi -- get_card --asset_code PSA86768374 >/dev/null 2>&1; then
  run "stellar contract invoke --id vault --source-account moi -- get_card --asset_code PSA86768374"
else
  run "stellar contract invoke --id vault --source-account moi --send=yes -- register_card --asset_code PSA86768374 --token CDRINTY4HMKL52OPNN3V5HXWIXRZXKZ4TKCMRWM24VVWVLUTH2ZWNHYZ --grader PSA --cert '\"86768374\"' --grade 10 --description '\"2022 Pokemon SWSH FR Lugia V #186 Silver Tempest\"' --photo_hash fea44be57a1038c49a0f092207bb31436c066f38060fccd1d2ec84842d9dad26"
fi
sleep 1

say "3. The buyer wants it for $PRICE_USDC USDC: the money goes into the escrow, not to the seller"
note "buyer USDC before: $(usdc "$COL")   seller: $(usdc "$VEN")"
run "stellar contract invoke --id escrow --source-account collectionneur --send=yes -- open --buyer $COL --seller $VEN --card PSA86768374 --amount $PRICE_UNITS --deadline_ledgers 120960"
T1=$(printf '%s' "$out" | grep -E '^[0-9]+$' | tail -1)
[ -n "$T1" ] || fail "open failed (does the buyer hold $((PRICE_USDC + TRAP_USDC)) USDC with a USDC trustline? Lower the amounts with PRICE_USDC= and TRAP_USDC=)"
note "trade id: $T1   buyer USDC now: $(usdc "$COL")"
sleep 1

say "4. The card is checked, the custodian releases: the contract pays the seller"
run "stellar contract invoke --id escrow --source-account moi --send=yes -- release --trade_id $T1"
note "seller USDC now: $(usdc "$VEN")"
sleep 1

say "5. Another buyer pays $TRAP_USDC USDC for the Zekrom, which already LEFT the vault (shipped)"
run "stellar contract invoke --id escrow --source-account collectionneur --send=yes -- open --buyer $COL --seller $VEN --card PSA137798077 --amount $TRAP_UNITS --deadline_ledgers 120960"
T2=$(printf '%s' "$out" | grep -E '^[0-9]+$' | tail -1)
[ -n "$T2" ] || fail "open failed for the second trade"
sleep 1

say "6. The custodian tries to release anyway: the contract refuses. No card in the vault, no payment."
run "stellar contract invoke --id escrow --source-account moi --send=yes -- release --trade_id $T2" || printf "  ${R}→ Error #6 CardNotInVault: the $TRAP_USDC USDC stay locked${N}\n"
sleep 2

say "7. So the buyer gets their money back"
run "stellar contract invoke --id escrow --source-account moi --send=yes -- refund --caller $MOI --trade_id $T2"
note "buyer USDC now: $(usdc "$COL")"
sleep 1

say "Every step above is a public transaction on Stellar."
printf "${G}escrow  https://stellar.expert/explorer/testnet/contract/%s${N}\n" "$ESCROW"
printf "${G}vault   https://stellar.expert/explorer/testnet/contract/%s${N}\n" "$VAULT"
sleep 4
