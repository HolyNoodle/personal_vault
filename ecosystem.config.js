module.exports = {
  apps: [
    {
      name: 'file-explorer-build',
      cwd: '/app',
      script: 'cargo',
      args: 'build --release --target wasm32-unknown-unknown -p file-explorer',
      interpreter: 'none',
      autorestart: false,
      watch: false,
      env: {},
      wait_ready: false,
    },
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
      },
      wait_ready: false,
      // PM2 does not natively support explicit process dependencies, but you can use pm2's "startOrRestart" with a custom script or use "pm2 start file-explorer-build && pm2 start backend" in your workflow.
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
