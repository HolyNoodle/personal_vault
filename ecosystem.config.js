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
        // Ensure DISPLAY is set for GUI applications so no applications try to render
        "DISPLAY": "",
        STORAGE_PATH: "../docker_data/storage",
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
