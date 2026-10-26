#!/usr/bin/env sh
# Anime le carnet d'ordres natif Stellar pour la carte PSA137798077 (testnet).
#
# - le dépositaire (émetteur) place un ordre de VENTE : 1 carte contre XLM
# - le collectionneur place un ordre d'ACHAT : 1 carte contre XLM
# Les deux ordres ne se croisent pas, le carnet affiche donc un prix d'achat,
# un prix de vente et un spread. Pour exécuter un échange, rapprochez les prix.
#
#   sh demo-web-page/seed-market.sh            # prix par défaut : vente 2100 XLM, achat 1900 XLM
#   ASK=2000 BID=1950 sh demo-web-page/seed-market.sh
set -eu

CODE="${CODE:-PSA137798077}"
ISSUER="$(stellar keys address moi)"
ASSET="$CODE:$ISSUER"
ONE=10000000                      # 1 unité = 1 carte (7 décimales)
ASK="${ASK:-2100}"                # prix de vente, en XLM par carte
BID="${BID:-1900}"                # prix d'achat, en XLM par carte

echo "— ordre de vente : moi vend 1 $CODE à $ASK XLM"
stellar tx new manage-sell-offer --source-account moi --network testnet \
  --selling "$ASSET" --buying native --amount "$ONE" --price "$ASK:1"

echo "— ordre d'achat : collectionneur achète 1 $CODE à $BID XLM"
stellar tx new manage-buy-offer --source-account collectionneur --network testnet \
  --selling native --buying "$ASSET" --amount "$ONE" --price "$BID:1"

echo "— carnet d'ordres (Horizon)"
curl -s "https://horizon-testnet.stellar.org/order_book?selling_asset_type=credit_alphanum12&selling_asset_code=$CODE&selling_asset_issuer=$ISSUER&buying_asset_type=native&limit=5" \
  | sed 's/,/,\n/g' | grep -E '"price"|"amount"' | head -8
