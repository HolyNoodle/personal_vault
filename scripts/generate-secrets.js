#!/usr/bin/env node

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');

console.log('🔐 Generating secure secrets...\n');

const secretsDir = path.join(__dirname, '..', 'secrets');

// Create secrets directory
if (!fs.existsSync(secretsDir)) {
  fs.mkdirSync(secretsDir, { recursive: true });
}

// Generate database password
const dbPassword = crypto.randomBytes(32).toString('base64');
const dbPasswordFile = path.join(secretsDir, 'db_password.txt');
fs.writeFileSync(dbPasswordFile, dbPassword);
fs.chmodSync(dbPasswordFile, 0o600);
console.log(`✅ Database password: ${dbPasswordFile}`);

// Generate Redis password
const redisPassword = crypto.randomBytes(32).toString('base64');
const redisPasswordFile = path.join(secretsDir, 'redis_password.txt');
fs.writeFileSync(redisPasswordFile, redisPassword);
fs.chmodSync(redisPasswordFile, 0o600);
console.log(`✅ Redis password: ${redisPasswordFile}`);

// Generate JWT secret
const jwtSecret = crypto.randomBytes(64).toString('base64');
const jwtSecretFile = path.join(secretsDir, 'jwt_secret.txt');
fs.writeFileSync(jwtSecretFile, jwtSecret);
fs.chmodSync(jwtSecretFile, 0o600);
console.log(`✅ JWT secret: ${jwtSecretFile}`);

console.log('\n✅ All secrets generated successfully!');
console.log('\n📝 Update your .env.production with:');
console.log(`   DB_PASSWORD=${dbPassword.substring(0, 20)}...`);
console.log(`   REDIS_PASSWORD=${redisPassword.substring(0, 20)}...`);
console.log(`   JWT_SECRET=${jwtSecret.substring(0, 20)}...`);
