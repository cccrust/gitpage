import { defineConfig } from '@playwright/test';

export default defineConfig({
    testDir: './specs',
    timeout: 60000,
    retries: 1,
    use: {
        baseURL: process.env.BASE_URL || 'http://localhost:8080',
        headless: true,
        screenshot: 'only-on-failure',
        trace: 'retain-on-failure',
    },
    webServer: {
        command: 'cd ../.. && cargo run',
        port: 8080,
        timeout: 30000,
        reuseExistingServer: true,
    },
});
