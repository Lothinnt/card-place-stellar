#!/usr/bin/env bash
# Live demo on Stellar testnet: the escrow refuses to pay for a card the vault doesn't hold,
# and pays the seller for a card it does. Every step prints its stellar.expert link and opens it.
#
#   sh demo/demo.sh          # needs the identities moi / collectionneur / vendeur and the aliases vault / escrow
#
# Identities: moi = Card Place (custodian), collectionneur = buyer, vendeur = seller.
set -u
cd "$(dirname "$0")/.."
export STELLAR_NETWORK=testnet

B='\033[1m'; D='\033[2m'; G='\033[32m'; R='\033[31m'; C='\033[36m'; N='\033[0m'
OPEN_LINKS=${OPEN_LINKS:-1}

say()  { printf "\n${B}%s${N}\n" "$*"; }
note() { printf "${D}%s${N}\n" "$*"; }
run()  { # prints the command with a typing effect, then runs it and shows the explorer link
  printf "${C}$ ${N}"; for ((i = 0; i < ${#1}; i++)); do printf '%s' "${1:$i:1}"; sleep 0.012; done; printf '\n'
  out=$(eval "$1" 2>&1); code=$?
  printf '%s\n' "$out" | grep -vE "^(ℹ️|📅|⚠️)" | grep -v "Simulation identified" | sed 's/^/  /'
  link=$(printf '%s\n' "$out" | grep -o 'https://stellar.expert[^ ]*' | head -1)
  if [ -n "$link" ] && [ "$OPEN_LINKS" = 1 ]; then open "$link"; sleep 6; osascript -e 'tell application "Terminal" to activate' >/dev/null; fi
  return $code
}

MOI=$(stellar keys address moi); COL=$(stellar keys address collectionneur); VEN=$(stellar keys address vendeur)
usdc() { curl -s "https://horizon-testnet.stellar.org/accounts/$1" | python3 -c "import sys,json; print([b['balance'] for b in json.load(sys.stdin)['balances'] if b.get('asset_code')=='USDC'][0])"; }

clear
say "Card Place — live on Stellar testnet"
note "custodian  $MOI"
note "buyer      $COL"
note "seller     $VEN"
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

say "3. The buyer wants it for 943 USDC: the money goes into the escrow, not to the seller"
note "buyer USDC before: $(usdc "$COL")   seller: $(usdc "$VEN")"
run "stellar contract invoke --id escrow --source-account collectionneur --send=yes -- open --buyer $COL --seller $VEN --card PSA86768374 --amount 9430000000 --deadline_ledgers 120960"
T1=$(printf '%s' "$out" | grep -E '^[0-9]+$' | tail -1)
note "trade id: $T1   buyer USDC now: $(usdc "$COL")"
sleep 1

say "4. The card is checked, the custodian releases: the contract pays the seller"
run "stellar contract invoke --id escrow --source-account moi --send=yes -- release --trade_id $T1"
note "seller USDC now: $(usdc "$VEN")"
sleep 1

say "5. Another buyer pays 50 USDC for the Zekrom, which already LEFT the vault (shipped)"
run "stellar contract invoke --id escrow --source-account collectionneur --send=yes -- open --buyer $COL --seller $VEN --card PSA137798077 --amount 500000000 --deadline_ledgers 120960"
T2=$(printf '%s' "$out" | grep -E '^[0-9]+$' | tail -1)
sleep 1

say "6. The custodian tries to release anyway: the contract refuses. No card in the vault, no payment."
run "stellar contract invoke --id escrow --source-account moi --send=yes -- release --trade_id $T2" || printf "  ${R}→ Error #6 CardNotInVault: the 50 USDC stay locked${N}\n"
sleep 2

say "7. So the buyer gets their money back"
run "stellar contract invoke --id escrow --source-account moi --send=yes -- refund --caller $MOI --trade_id $T2"
note "buyer USDC now: $(usdc "$COL")"
sleep 1

say "Every step above is a public transaction on Stellar."
printf "${G}escrow  https://stellar.expert/explorer/testnet/contract/CBUFJVVLC2TY7MDKEQKDAFP7TXPKP6MEDW2XWESEKJFG5V6ZJUEVHNWG${N}\n"
printf "${G}vault   https://stellar.expert/explorer/testnet/contract/CDN5OOWTOYKEXMH5CDG7APQRQEHGFRKWQEAHQR5XR3643YOM2YEMEMRO${N}\n"
sleep 4
