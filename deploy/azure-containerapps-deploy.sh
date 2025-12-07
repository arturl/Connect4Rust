#!/usr/bin/env bash
set -euo pipefail

# Zero-arg deploy to Azure Container Apps (consumption, scales to zero).
# Prereqs: az CLI logged in with containerapp extension; docker already pushed image to ACR (script builds/pushes).
#
# Resources (idempotent):
#   RG: Connect4Rust
#   ACR: connect4rustacr
#   Container App: connect4rust-app
#   Location: eastus

RG="Connect4Rust"
LOCATION="eastus"
ACR="connect4rustacr"
APP="connect4rust-app"
IMAGE="$ACR.azurecr.io/connect4rust:latest"

echo "Ensuring resource group $RG..."
az group create -n "$RG" -l "$LOCATION" >/dev/null

echo "Ensuring ACR $ACR..."
az acr create -n "$ACR" -g "$RG" --sku Basic --admin-enabled true >/dev/null 2>&1 || true
az acr login -n "$ACR"

echo "Building linux/amd64 image in ACR (native build, no emulation)..."
az acr build --registry "$ACR" --image connect4rust:latest --platform linux/amd64 .

echo "Deploying Container App $APP..."
az containerapp up \
  --name "$APP" \
  --resource-group "$RG" \
  --location "$LOCATION" \
  --image "$IMAGE" \
  --target-port 3000 \
  --ingress external \
  --registry-server "$ACR.azurecr.io" \
  --registry-username "$(az acr credential show -n "$ACR" --query username -o tsv)" \
  --registry-password "$(az acr credential show -n "$ACR" --query passwords[0].value -o tsv)" \
  >/dev/null

echo "Done. Fetch URL with:"
echo "az containerapp show -n $APP -g $RG --query properties.configuration.ingress.fqdn -o tsv"
