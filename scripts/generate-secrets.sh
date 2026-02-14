#!/bin/bash

# Create secrets directory
mkdir -p secrets

# Generate secure database password
echo "Generating secure database password..."
openssl rand -base64 32 > secrets/db_password.txt

# Generate Redis password
echo "Generating Redis password..."
openssl rand -base64 32 > secrets/redis_password.txt

# Set permissions
chmod 600 secrets/*.txt

echo "✅ Secrets generated!"
echo ""
echo "Update .env.production with:"
echo "  DB_PASSWORD=\$(cat secrets/db_password.txt)"
echo "  REDIS_PASSWORD=\$(cat secrets/redis_password.txt)"
