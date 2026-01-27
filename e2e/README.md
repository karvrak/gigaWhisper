# E2E Tests - GigaWhisper

End-to-end tests using Playwright for the Tauri application.

## Prerequisites

1. Build the Tauri app first:
   ```bash
   pnpm tauri build
   ```

2. Install Playwright:
   ```bash
   pnpm add -D @playwright/test
   npx playwright install chromium
   ```

## Running Tests

```bash
# Run all e2e tests
pnpm test:e2e

# Run with UI mode (interactive)
pnpm test:e2e:ui

# Run specific test file
pnpm test:e2e e2e/tests/app.spec.ts
```

## Structure

```
e2e/
├── README.md           # This file
├── setup/
│   ├── global-setup.ts    # Runs before all tests (starts app)
│   └── global-teardown.ts # Runs after all tests (cleanup)
├── fixtures/
│   └── tauri-fixture.ts   # Custom Playwright fixture for Tauri
└── tests/
    └── app.spec.ts        # Main application tests
```

## Writing Tests

Use the custom Tauri fixture:

```typescript
import { test, expect } from '../fixtures/tauri-fixture';

test('example test', async ({ app }) => {
  // app is the Tauri window
  await expect(app.locator('h1')).toContainText('GigaWhisper');
});
```

## Notes

- Tests run against the built app, not dev mode
- Tests are sequential (not parallel) due to single app instance
- Screenshots and traces are saved on failure
- CI will retry failed tests twice
