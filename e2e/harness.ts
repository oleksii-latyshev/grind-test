import { mockIPC } from '@tauri-apps/api/mocks';

declare global {
  interface Window {
    fakeBackend: (command: string, args: unknown) => Promise<unknown>;
  }
}

// Installed before the app is imported, so not even the first `invoke` reaches for Tauri.
mockIPC((command, args) => window.fakeBackend(command, args), { shouldMockEvents: true });
await import('../src/main.tsx');
