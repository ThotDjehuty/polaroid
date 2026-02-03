#!/bin/bash
set -e

echo "☁️  Deploying Polaroid to Azure Functions"
echo "========================================"

# Configuration
RESOURCE_GROUP="${RESOURCE_GROUP:-QANT-EXPE-BENCH}"
LOCATION="${LOCATION:-switzerlandnorth}"
FUNCTION_APP="${FUNCTION_APP:-hftlab-polaroid-func}"
STORAGE_ACCOUNT="${STORAGE_ACCOUNT:-hftlabpolaroidstorage}"

# Check dependencies
if ! command -v az &> /dev/null; then
    echo "❌ Azure CLI not found. Please install: https://aka.ms/azure-cli"
    exit 1
fi

echo "ℹ️  Azure Functions Core Tools not required (using zip deployment)"

# Check if logged in
if ! az account show &> /dev/null; then
    echo "❌ Not logged in to Azure. Run: az login"
    exit 1
fi

echo "✅ Azure CLI ready"
echo "   Subscription: $(az account show --query name -o tsv)"
echo ""

# Check if binary exists
BINARY_PATH="serverless/target/release/polaroid-http"
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found at $BINARY_PATH"
    echo "   Run: ./scripts/build-serverless.sh"
    exit 1
fi
echo "✅ Binary found: $BINARY_PATH ($(du -h $BINARY_PATH | cut -f1))"

echo "📦 Creating Azure resources..."

# Create resource group (if not exists)
if ! az group show --name $RESOURCE_GROUP &> /dev/null; then
    echo "   Creating resource group: $RESOURCE_GROUP"
    az group create --name $RESOURCE_GROUP --location $LOCATION
else
    echo "   ✅ Resource group exists: $RESOURCE_GROUP"
fi

# Create storage account (if not exists)
if ! az storage account show --name $STORAGE_ACCOUNT --resource-group $RESOURCE_GROUP &> /dev/null; then
    echo "   Creating storage account: $STORAGE_ACCOUNT"
    az storage account create \
        --name $STORAGE_ACCOUNT \
        --resource-group $RESOURCE_GROUP \
        --location $LOCATION \
        --sku Standard_LRS
else
    echo "   ✅ Storage account exists: $STORAGE_ACCOUNT"
fi

# Create function app (consumption plan)
if ! az functionapp show --name $FUNCTION_APP --resource-group $RESOURCE_GROUP &> /dev/null; then
    echo "   Creating function app: $FUNCTION_APP"
    az functionapp create \
        --name $FUNCTION_APP \
        --resource-group $RESOURCE_GROUP \
        --consumption-plan-location $LOCATION \
        --runtime custom \
        --os-type Linux \
        --functions-version 4 \
        --storage-account $STORAGE_ACCOUNT
else
    echo "   ✅ Function app exists: $FUNCTION_APP"
fi

echo ""
echo "🚀 Deploying function with Custom Handler..."

# Navigate to serverless directory
cd "$(dirname "$0")/../serverless"

# Prepare deployment package
echo "📦 Preparing deployment package..."
rm -rf azure-deploy
mkdir -p azure-deploy

# Copy Azure Functions configuration
cp -r azure-functions/* azure-deploy/

# Copy binary to root of deployment package
cp target/release/polaroid-http azure-deploy/
chmod +x azure-deploy/polaroid-http

# Verify package structure
echo "📂 Deployment package structure:"
ls -lh azure-deploy/
ls -lh azure-deploy/PolaroidApi/

# Create zip for deployment
echo "📦 Creating deployment zip..."
cd azure-deploy
zip -r ../azure-deploy.zip . > /dev/null
cd ..

# Deploy using zip deployment
echo "📤 Uploading to Azure..."
az functionapp deployment source config-zip \
  --resource-group $RESOURCE_GROUP \
  --name $FUNCTION_APP \
  --src azure-deploy.zip

# Configure app settings for Custom Handler
echo "⚙️  Configuring app settings..."
az functionapp config appsettings set \
  --name $FUNCTION_APP \
  --resource-group $RESOURCE_GROUP \
  --settings \
    "FUNCTIONS_WORKER_RUNTIME=custom" \
    "RUST_LOG=info" \
    "JWT_SECRET=${JWT_SECRET:-dev-secret-change-in-production}"

echo ""
echo "✅ Deployment complete!"
FUNCTION_URL=$(az functionapp show --name $FUNCTION_APP --resource-group $RESOURCE_GROUP --query defaultHostName -o tsv)
echo "📍 Function URL: https://$FUNCTION_URL"
echo ""
echo "🧪 Test endpoints:"
echo "   Health:   curl https://$FUNCTION_URL/api/health"
echo "   Discover: curl -X POST https://$FUNCTION_URL/api/discover-pairs -H 'Content-Type: application/json' -d '{\"symbols\":[\"AAPL\",\"MSFT\"]}'"
echo "   Metrics:  curl https://$FUNCTION_URL/metrics"
echo ""
echo "📊 Monitor logs:"
echo "   az functionapp log tail --name $FUNCTION_APP --resource-group $RESOURCE_GROUP"
echo ""
echo "💰 Estimated cost: \$0-5/month (Consumption plan, auto-scale-to-zero)"
