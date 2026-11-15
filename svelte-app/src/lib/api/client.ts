// =============================================================================
// TYPED REST API CLIENT WRAPPER
// =============================================================================
//
// Wraps generated `API_ENDPOINTS` with path interpolation, query encoding,
// standardized `ApiResponse<T>` error handling, and `X-Hot-Support: 1`
// propagation for isolated Admin Hot Support sessions.

import { browser } from '$app/env';
import { API_ENDPOINTS, type ApiEndpointDef } from '#lib/api/endpoints.generated.ts';
import type { ApiResponse } from '#lib/api/types.generated.ts';

export { API_ENDPOINTS };

export interface RequestOptions<B = unknown> {
  params?: Record<string, string | number>;
  query?: Record<string, string | number | boolean | null | undefined>;
  body?: B;
  headers?: Record<string, string>;
}

export function isHotSupportActive(): boolean {
  if (!browser) return false;
  if (window.__APP_BOOTSTRAP__?.is_hot_support) return true;
  return new URLSearchParams(window.location.search).get('_hs') === '1';
}

export function buildEndpointUrl(
  def: ApiEndpointDef,
  params?: Record<string, string | number>,
  query?: Record<string, string | number | boolean | null | undefined>
): string {
  let path = def.path;
  if (params) {
    for (const [key, val] of Object.entries(params)) {
      path = path.replace(`{${key}}`, encodeURIComponent(String(val)));
    }
  }

  if (query) {
    const sp = new URLSearchParams();
    for (const [key, val] of Object.entries(query)) {
      if (val !== null && val !== undefined && val !== '') {
        sp.set(key, String(val));
      }
    }
    const qs = sp.toString();
    if (qs) {
      path = `${path}?${qs}`;
    }
  }

  return path;
}

export async function apiRequest<T, B = unknown>(
  def: ApiEndpointDef,
  options: RequestOptions<B> = {}
): Promise<ApiResponse<T>> {
  const url = buildEndpointUrl(def, options.params, options.query);
  const headers: Record<string, string> = {
    ...(options.headers ?? {})
  };

  if (options.body !== undefined) {
    headers['Content-Type'] = 'application/json';
  }
  if (isHotSupportActive()) {
    headers['X-Hot-Support'] = '1';
  }

  try {
    const response = await fetch(url, {
      method: def.method,
      headers,
      credentials: 'same-origin',
      body: options.body !== undefined ? JSON.stringify(options.body) : undefined
    });

    const contentType = response.headers.get('content-type') ?? '';
    if (contentType.includes('application/json')) {
      return (await response.json()) as ApiResponse<T>;
    }

    return {
      success: response.ok,
      message: response.ok ? 'OK' : 'INTERNAL_ERROR',
      status: response.ok
        ? { Success: 'Ok' }
        : { Error: 'InternalServerError' },
      data: null
    };
  } catch {
    return {
      success: false,
      message: 'NETWORK_ERROR',
      status: { Error: 'ServiceUnavailable' },
      data: null
    };
  }
}
