/**
 * API configuration helpers for centralized base URL management
 */

/**
 * Get the API base URL, preferring environment variable over fallback
 * @returns API base URL string
 */
export function getApiBase(): string {
  // Prefer NEXT_PUBLIC_API_BASE env var
  const envBase = process.env.NEXT_PUBLIC_API_BASE;
  if (envBase) {
    return envBase;
  }

  // Fallback to window.location.origin only in browser environment
  if (typeof window !== 'undefined') {
    return window.location.origin;
  }

  // Default fallback for SSR/build time
  return '';
}

/**
 * Get the WebSocket base URL, preferring environment variable over fallback
 * @returns WebSocket base URL string
 */
export function getWsBase(): string {
  // Prefer NEXT_PUBLIC_WS_BASE env var
  const envBase = process.env.NEXT_PUBLIC_WS_BASE;
  if (envBase) {
    return envBase;
  }

  // Fallback to window.location.origin with ws protocol conversion
  if (typeof window !== 'undefined') {
    const origin = window.location.origin;
    return origin.replace(/^http/, 'ws');
  }

  // Default fallback for SSR/build time
  return '';
}