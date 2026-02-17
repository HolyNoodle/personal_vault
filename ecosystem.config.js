module.exports = {
  apps: [
    {
      name: 'backend',
      cwd: '/app/backend',
      script: 'cargo',
      args: 'watch -w src -w Cargo.toml -x run',
      interpreter: 'none',
      autorestart: true,
      watch: false,
      env: {
        "DISPLAY": "",
      }
    },
    {
      name: 'frontend',
      cwd: '/app/frontend/web',
      script: 'bash',
      args: '-c "npm install && npm run dev -- --host 0.0.0.0"',
      interpreter: 'none',
      autorestart: false,
      watch: false,
    },
  ],
}
