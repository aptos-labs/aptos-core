import { Buffer } from 'buffer';

// Polyfill Buffer
window.Buffer = Buffer;

// Add other polyfills if needed
if (typeof (window as any).global === 'undefined') {
  (window as any).global = window;
}

if (typeof (window as any).process === 'undefined') {
  (window as any).process = { env: {} };
}
