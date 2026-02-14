#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const env = process.argv[2] || 'development';

console.log(`🔧 Setting up ${env} environment...`);

const sourceFile = path.join(__dirname, '..', `.env.${env}`);
const targetFile = path.join(__dirname, '..', '.env');

if (!fs.existsSync(sourceFile)) {
  console.error(`❌ ${sourceFile} not found!`);
  console.log(`\nCreate it by copying the template:`);
  console.log(`  cp .env.default .env.${env}`);
  console.log(`  # Then edit .env.${env} with your settings`);
  process.exit(1);
}

fs.copyFileSync(sourceFile, targetFile);
console.log(`✅ Copied ${sourceFile} to ${targetFile}`);

// Create necessary directories
const dirs = ['storage', 'logs', 'secrets'];
dirs.forEach(dir => {
  const dirPath = path.join(__dirname, '..', dir);
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
    console.log(`✅ Created ${dir}/ directory`);
  }
});

if (env === 'production') {
  const dbPasswordFile = path.join(__dirname, '..', 'secrets', 'db_password.txt');
  const redisPasswordFile = path.join(__dirname, '..', 'secrets', 'redis_password.txt');
  
  if (!fs.existsSync(dbPasswordFile) || !fs.existsSync(redisPasswordFile)) {
    console.warn(`\n⚠️  Warning: Secret files not found!`);
    console.log(`Run: npm run secrets:generate`);
  }
}

console.log(`\n✅ Environment setup complete!`);
