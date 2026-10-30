const { defineConfig } = require('@playwright/test');
module.exports = defineConfig({ testDir: './tests/terminal', workers: 1, projects: [{name: 'webkit', use: {browserName: 'webkit', viewport: {width: 1000, height: 600}}}] });
