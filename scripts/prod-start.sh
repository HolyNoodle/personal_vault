#!/bin/bash
set -e

echo "🚀 Starting Production Environment..."
echo ""

# Check if .env.production exists
if [ ! -f .env.production ]; then
    echo "❌ .env.production not found!"
    exit 1
fi

# Check secrets
if [ ! -f secrets/db_password.txt ]; then
    echo "❌ Database password secret not found!"
    echo "Create: echo 'your-secure-password' > secrets/db_password.txt"
    exit 1
fi

# Copy to .env
cp .env.production .env

# Build and start
echo "Building production images..."
docker-compose -f docker-compose.prod.yml build

echo "Starting services..."
docker-compose -f docker-compose.prod.yml up -d

echo ""
echo "⏳ Waiting for services..."
sleep 10

echo ""
echo "✅ Production environment is ready!"
echo ""
echo "Services:"
echo "  🌐 Application:    https://sandbox.example.com"
echo "  📊 HAProxy Stats: http://localhost:8404/stats"
echo ""
echo "Logs:"
echo "  docker-compose -f docker-compose.prod.yml logs -f backend"
echo "  docker-compose -f docker-compose.prod.yml logs -f haproxy"
echo ""
echo "Stop:"
echo "  docker-compose -f docker-compose.prod.yml down"
