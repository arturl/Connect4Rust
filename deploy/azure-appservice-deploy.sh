#!/usr/bin/env bash
set -euo pipefail

# Zero-arg deploy to Azure App Service (Linux, Free tier).
# Prereqs: az CLI logged in; docker already pushed image to ACR (script builds/pushes).
#
# Resources (idempotent):
#   RG: Connect4Rust2
#   ACR: connect4rustacr2
#   App Service Plan: connect4rust-plan (Free tier)
#   Web App: connect4rust (https://connect4rust.azurewebsites.net)
#   Location: eastus

RG="Connect4Rust2"
LOCATION="eastus"
ACR="connect4rustacr2"
PLAN="connect4rust-plan"
APP="connect4rust"
IMAGE="$ACR.azurecr.io/connect4rust:latest"

echo "Ensuring resource group $RG..."
az group create -n "$RG" -l "$LOCATION" >/dev/null

echo "Ensuring ACR $ACR..."
az acr create -n "$ACR" -g "$RG" --sku Basic --admin-enabled true >/dev/null 2>&1 || true
az acr login -n "$ACR"

echo "Building linux/amd64 image in ACR (native build, no emulation)..."
# Remove --no-cache if you want faster builds with layer caching
az acr build --registry "$ACR" --image connect4rust:latest --platform linux/amd64 .

echo "Ensuring App Service Plan $PLAN (Free tier)..."
az appservice plan create \
  --name "$PLAN" \
  --resource-group "$RG" \
  --sku F1 \
  --is-linux \
  --location "$LOCATION" >/dev/null 2>&1 || true

echo "Deploying Web App $APP..."
# Create or update the web app
az webapp create \
  --name "$APP" \
  --resource-group "$RG" \
  --plan "$PLAN" \
  --deployment-container-image-name "$IMAGE" >/dev/null 2>&1 || true

# Configure container registry credentials
ACR_USER=$(az acr credential show -n "$ACR" --query username -o tsv)
ACR_PASS=$(az acr credential show -n "$ACR" --query passwords[0].value -o tsv)

az webapp config container set \
  --name "$APP" \
  --resource-group "$RG" \
  --container-image-name "$IMAGE" \
  --container-registry-url "https://$ACR.azurecr.io" \
  --container-registry-user "$ACR_USER" \
  --container-registry-password "$ACR_PASS" >/dev/null

# Set the container port
az webapp config appsettings set \
  --name "$APP" \
  --resource-group "$RG" \
  --settings WEBSITES_PORT=3000 >/dev/null

echo "Restarting web app to apply changes..."
az webapp restart --name "$APP" --resource-group "$RG" >/dev/null

echo ""
echo "✅ Deployment complete!"
echo "URL: https://$APP.azurewebsites.net"
