#!/bin/bash
set -e

echo "🚀 Starting Development Environment..."
echo ""

# Check if .env.development exists
if [ ! -f .env.development ]; then
    echo "❌ .env.development not found!"
    exit 1
fi

# Copy to .env for docker-compose
cp .env.development .env

# Create storage directory
mkdir -p storage logs

# Start services
echo "Starting Docker services..."
docker compose -f docker-compose.dev.yml up -d

echo ""
echo "⏳ Waiting for services to be healthy..."
sleep 5

# Wait for postgres
echo "Checking PostgreSQL..."
until docker exec sandbox-postgres-dev pg_isready -U sandbox_user -d sandbox_dev; do
    echo "Waiting for PostgreSQL..."
    sleep 2
done

echo ""
echo "✅ Development environment is ready!"
echo ""
echo "Services:"
echo "  🦀 Backend API:    http://localhost:8080"
echo "  ⚛️  Frontend:       http://localhost:5173"
echo "  🗄️  PostgreSQL:     localhost:5432"
echo "  📮 Redis:          localhost:6379"
echo "  📧 Mailhog UI:     http://localhost:8025"
echo ""
echo "Logs:"
echo "  docker compose -f docker-compose.dev.yml logs -f backend"
echo "  docker compose -f docker-compose.dev.yml logs -f frontend"
echo ""
echo "Stop:"
echo "  ./scripts/dev-stop.sh"
